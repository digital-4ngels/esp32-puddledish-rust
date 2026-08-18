#pragma once

#define LCD_H_RES 466
#define LCD_V_RES 466
#define BOX_W ((float)LCD_H_RES)
#define BOX_H ((float)LCD_V_RES)
#define BOX_D 75.0f
#define BOX_CORNER_R (BOX_W * 0.5f)

#define PX_PER_METER 12677.0f
#define PX_PER_MM (PX_PER_METER / 1000.0f)
#define BOX_BACK_FILLET_MM 2.0f
#define BOX_BACK_FILLET (BOX_BACK_FILLET_MM * PX_PER_MM)
#define BOX_FRONT_FILLET (BOX_BACK_FILLET * 0.25f)

#define PARTICLE_MAX 1000
#define PARTICLE_COUNT 900
#define REST_SPACING 16.0f

#define TIME_SCALE 0.068f
#define SIM_DT_MAX 0.0022f
#define GRAVITY_MPS2 9.81f
#define GRAVITY_GAIN 1.8f
#define SMOOTH_RADIUS 28.0f
#define SUBSTEPS 1
#ifndef K_PRESSURE
#define K_PRESSURE 400000.0f
#endif
#ifndef K_NEAR_PRESSURE
#define K_NEAR_PRESSURE 800000.0f
#endif
#ifndef MAX_DISPLACEMENT
#define MAX_DISPLACEMENT 4.0f
#endif
#define WALL_JITTER 0.0f
#define VISC_SIGMA 45.0f
#define VISC_BETA 0.03f
#define WALL_RESTITUTION 0.0f
#define WALL_FRICTION 0.96f
#ifndef WALL_SPEED_CAP
#define WALL_SPEED_CAP 0.0f
#endif
#ifndef RISE_CAP
#define RISE_CAP 0.0f
#endif
#define ROTATION_GAIN 1.0f
