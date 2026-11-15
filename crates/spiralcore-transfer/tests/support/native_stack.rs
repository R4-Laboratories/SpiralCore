//! Test observer for this thread's actual stack; no stack memory is read.
#[cfg(target_os = "linux")]
pub fn current() -> (usize, Option<usize>) {
    let mut attributes = std::mem::MaybeUninit::<libc::pthread_attr_t>::uninit();
    // SAFETY: pthread_self names this live thread; the out pointer has the
    // native pthread_attr_t layout. Read it only after successful initialization.
    let created =
        unsafe { libc::pthread_getattr_np(libc::pthread_self(), attributes.as_mut_ptr()) };
    assert_eq!(created, 0);
    // SAFETY: successful pthread_getattr_np initialized attributes above.
    let mut attributes = unsafe { attributes.assume_init() };
    let mut base = std::ptr::null_mut();
    let mut bytes = 0;
    let mut guard = 0;
    // SAFETY: attributes is live/initialized; all three outputs are writable.
    // The returned base is checked but never dereferenced or published.
    let (stack_result, guard_result, destroyed) = unsafe {
        let stack = libc::pthread_attr_getstack(&attributes, &mut base, &mut bytes);
        let guard = libc::pthread_attr_getguardsize(&attributes, &mut guard);
        let destroyed = libc::pthread_attr_destroy(&mut attributes);
        (stack, guard, destroyed)
    };
    assert_eq!((stack_result, guard_result, destroyed), (0, 0, 0));
    assert!(!base.is_null());
    (bytes, Some(guard))
}

#[cfg(target_os = "macos")]
pub fn current() -> (usize, Option<usize>) {
    // SAFETY: query this live thread's native stack size. No returned address,
    // foreign thread, mutable state or key material is accessed.
    let bytes = unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) };
    assert_ne!(bytes, 0);
    // Darwin provides no corresponding current-thread guard-size receipt here.
    (bytes, None)
}
