#pragma once

typedef int SemaphoreHandle_t;
#define portMAX_DELAY 0
#define xSemaphoreCreateMutex() (1)
void c_lock(void);
void c_unlock(void);
#define xSemaphoreTake(s, t) c_lock()
#define xSemaphoreGive(s) c_unlock()
