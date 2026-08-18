#include "imu.h"

static float s_down[3] = {0.0f, 1.0f, 0.0f};

void sim_set_down(float x, float y, float z)
{
    s_down[0] = x;
    s_down[1] = y;
    s_down[2] = z;
}

void imu_down(float out[3])
{
    out[0] = s_down[0];
    out[1] = s_down[1];
    out[2] = s_down[2];
}
