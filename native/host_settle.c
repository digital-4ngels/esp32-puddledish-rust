/* Host-only: does the puddle come to rest, or does the glass keep kicking? */
#include "config.h"
#include "imu.h"
#include "sim.h"

#include <math.h>
#include <stdio.h>

int main(void)
{
    const float g_px = GRAVITY_GAIN * GRAVITY_MPS2 * PX_PER_METER;
    sim_forces_t f = {
        .gravity = {0.0f, g_px, 0.0f},
        .omega = {0, 0, 0},
        .alpha = {0, 0, 0},
    };
    const float dt = 0.025f;
    const float cx = BOX_W * 0.5f;
    const float cy = BOX_H * 0.5f;
    const float rim = BOX_CORNER_R - 5.0f - 2.0f;

    sim_set_down(0.0f, 1.0f, 0.0f);
    sim_init();
    sim_reset();

    sim_particle_view_t view[PARTICLE_MAX];
    printf("TIME=%.3f REST=%.2f JITTER=%.2f g=%.0f\n",
           (double)TIME_SCALE, (double)WALL_RESTITUTION, (double)WALL_JITTER,
           (double)g_px);

    for (int step = 0; step <= 400; step++) {
        if (step > 0) {
            sim_step(dt, &f);
        }
        if (step % 50 != 0) {
            continue;
        }
        int n = sim_snapshot(view, PARTICLE_MAX);
        sim_stats_t st;
        sim_stats(&st);
        float comx = 0, comy = 0, comz = 0;
        int rim_n = 0, front_n = 0, back_n = 0;
        for (int i = 0; i < n; i++) {
            comx += view[i].x;
            comy += view[i].y;
            comz += view[i].z;
            float dx = view[i].x - cx;
            float dy = view[i].y - cy;
            if (dx * dx + dy * dy > rim * rim) {
                rim_n++;
            }
            if (view[i].z < 12.0f) {
                front_n++;
            }
            if (view[i].z > BOX_D - 12.0f) {
                back_n++;
            }
        }
        float inv = n ? 1.0f / (float)n : 0.0f;
        printf(
            "t=%5.2f n=%d rho=%.2f/%.2f speed=%.1f/%.0f com=(%.0f,%.0f,%.1f) rim=%d front=%d back=%d hits_fb=%d/%d clamp=%d pairs=%d\n",
            (double)(step * dt), n, (double)st.mean_density, (double)st.rest_density,
            (double)st.mean_speed, (double)st.max_speed,
            (double)(comx * inv), (double)(comy * inv), (double)(comz * inv),
            rim_n, front_n, back_n, st.front_hits, st.back_hits, st.clamped,
            st.pairs);
    }
    return 0;
}
