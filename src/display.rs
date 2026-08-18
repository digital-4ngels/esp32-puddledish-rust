//! CO5300 QSPI. Zwei DMA-Bänder, Draw überlappt Transfer — wie die C-Firmware.

use embassy_time::{Duration, Timer};
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::dma_buffers;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::{DMA_CH0, GPIO4, GPIO5, GPIO6, GPIO7, GPIO12, GPIO38, GPIO39, SPI2};
use esp_hal::spi::master::{Address, Command, Config, DataMode, Spi, SpiDma, SpiDmaTransfer};
use esp_hal::spi::Mode;
use esp_hal::time::Rate;
use esp_hal::Blocking;

pub use crate::config::{
    BAND_COUNT, BAND_ROWS, LCD_COL_OFFSET as COL_OFFSET, LCD_H_RES as LCD_H, LCD_V_RES as LCD_V,
};

const BAND_BYTES: usize = LCD_H as usize * BAND_ROWS * 2;
const OP_CMD: u16 = 0x02;
const OP_COLOR: u16 = 0x32;

pub struct Display<'d> {
    spi: Option<SpiDma<'d, Blocking>>,
    pixel: [Option<DmaTxBuf>; 2],
    cmd_buf: Option<DmaTxBuf>,
    inflight: Option<SpiDmaTransfer<'d, Blocking, DmaTxBuf>>,
    inflight_idx: usize,
    _rx: DmaRxBuf,
    _rst: Output<'d>,
}

impl<'d> Display<'d> {
    pub async fn init(
        spi2: SPI2<'d>,
        dma: DMA_CH0<'d>,
        pclk: GPIO38<'d>,
        cs: GPIO12<'d>,
        rst: GPIO39<'d>,
        d0: GPIO4<'d>,
        d1: GPIO5<'d>,
        d2: GPIO6<'d>,
        d3: GPIO7<'d>,
    ) -> Self {
        let mut rst = Output::new(rst, Level::High, OutputConfig::default());
        rst.set_low();
        Timer::after(Duration::from_millis(10)).await;
        rst.set_high();
        Timer::after(Duration::from_millis(150)).await;

        let (rx_buffer, rx_descriptors, cmd_buffer, cmd_descriptors) = dma_buffers!(64, 32);
        let (_, _, tx0, td0) = dma_buffers!(8, BAND_BYTES);
        let (_, _, tx1, td1) = dma_buffers!(8, BAND_BYTES);

        let dma_rx = DmaRxBuf::new(rx_descriptors, rx_buffer).expect("dma rx");
        let cmd_buf = DmaTxBuf::new(cmd_descriptors, cmd_buffer).expect("cmd tx");
        let pix0 = DmaTxBuf::new(td0, tx0).expect("pix0");
        let pix1 = DmaTxBuf::new(td1, tx1).expect("pix1");

        let spi = Spi::new(
            spi2,
            Config::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(Mode::_0),
        )
        .expect("spi")
        .with_sck(pclk)
        .with_sio0(d0)
        .with_sio1(d1)
        .with_sio2(d2)
        .with_sio3(d3)
        .with_cs(cs)
        .with_dma(dma);

        let mut this = Self {
            spi: Some(spi),
            pixel: [Some(pix0), Some(pix1)],
            cmd_buf: Some(cmd_buf),
            inflight: None,
            inflight_idx: 0,
            _rx: dma_rx,
            _rst: rst,
        };
        this.cmd(0x36, &[0x00]);
        this.cmd(0x3A, &[0x55]);
        this.cmd(0xFE, &[]);
        this.cmd(0xC4, &[0x80]);
        this.cmd(0x35, &[]);
        Timer::after(Duration::from_millis(10)).await;
        this.cmd(0x53, &[0x20]);
        Timer::after(Duration::from_millis(10)).await;
        this.cmd(0x51, &[0xFF]);
        Timer::after(Duration::from_millis(10)).await;
        this.cmd(0x63, &[0xFF]);
        Timer::after(Duration::from_millis(10)).await;
        this.cmd(0x11, &[]);
        Timer::after(Duration::from_millis(60)).await;
        this.cmd(0x29, &[]);
        this
    }

    fn wait_dma(&mut self) {
        if let Some(t) = self.inflight.take() {
            let (spi, buf) = t.wait();
            self.spi = Some(spi);
            self.pixel[self.inflight_idx] = Some(buf);
        }
    }

    fn write(
        &mut self,
        mode: DataMode,
        op: u16,
        lcd_cmd: u8,
        data: &[u8],
    ) {
        self.wait_dma();
        let mut cmd_buf = self.cmd_buf.take().expect("cmd");
        let n = data.len().min(cmd_buf.as_mut_slice().len());
        cmd_buf.as_mut_slice()[..n].copy_from_slice(&data[..n]);
        let spi = self.spi.take().expect("spi");
        match spi.half_duplex_write(
            mode,
            Command::_8Bit(op, DataMode::Single),
            Address::_24Bit((lcd_cmd as u32) << 8, DataMode::Single),
            0,
            n,
            cmd_buf,
        ) {
            Ok(t) => {
                let (spi, cmd_buf) = t.wait();
                self.spi = Some(spi);
                self.cmd_buf = Some(cmd_buf);
            }
            Err((_, spi, cmd_buf)) => {
                self.spi = Some(spi);
                self.cmd_buf = Some(cmd_buf);
            }
        }
    }

    fn cmd(&mut self, lcd_cmd: u8, data: &[u8]) {
        self.write(DataMode::Single, OP_CMD, lcd_cmd, data);
    }

    fn window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        let xs = x0 + COL_OFFSET;
        let xe = x1 + COL_OFFSET;
        self.cmd(
            0x2A,
            &[
                (xs >> 8) as u8,
                xs as u8,
                (xe >> 8) as u8,
                xe as u8,
            ],
        );
        self.cmd(
            0x2B,
            &[
                (y0 >> 8) as u8,
                y0 as u8,
                (y1 >> 8) as u8,
                y1 as u8,
            ],
        );
    }

    pub fn band_pixels(&mut self, idx: usize) -> &mut [u16] {
        let bytes = self.pixel[idx].as_mut().expect("band").as_mut_slice();
        unsafe {
            core::slice::from_raw_parts_mut(bytes.as_mut_ptr() as *mut u16, bytes.len() / 2)
        }
    }

    pub fn start_flush(&mut self, y0: u16, rows: u16, idx: usize) {
        if rows == 0 {
            return;
        }
        self.wait_dma();
        self.window(0, y0, LCD_H - 1, y0 + rows - 1);
        let nbytes = LCD_H as usize * rows as usize * 2;
        let spi = self.spi.take().expect("spi");
        let buf = self.pixel[idx].take().expect("pix");
        match spi.half_duplex_write(
            DataMode::Quad,
            Command::_8Bit(OP_COLOR, DataMode::Single),
            Address::_24Bit((0x2C_u32) << 8, DataMode::Single),
            0,
            nbytes,
            buf,
        ) {
            Ok(t) => {
                self.inflight = Some(t);
                self.inflight_idx = idx;
            }
            Err((_, spi, buf)) => {
                self.spi = Some(spi);
                self.pixel[idx] = Some(buf);
            }
        }
    }

    pub fn finish_frame(&mut self) {
        self.wait_dma();
    }
}
