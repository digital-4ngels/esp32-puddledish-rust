static volatile int s_lock;

void c_lock(void)
{
    while (__sync_lock_test_and_set(&s_lock, 1)) {
    }
}

void c_unlock(void)
{
    __sync_lock_release(&s_lock);
}
