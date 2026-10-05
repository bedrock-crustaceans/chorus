pub fn read_available(buffer: &mut [u8]) -> Option<usize> {
    let mut poll = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    if ready <= 0 || poll.revents & (libc::POLLIN | libc::POLLHUP) == 0 {
        return if poll.revents & libc::POLLNVAL != 0 { None } else { Some(0) };
    }
    let read = unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };
    if read <= 0 { None } else { Some(read as usize) }
}
