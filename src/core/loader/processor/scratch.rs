use super::WorkerState;

pub(super) fn take_scratch_buffer(st: &mut WorkerState, size: usize) -> Vec<u8> {
    if let Some(mut buf) = st.scratch_pool.pop() {
        if buf.len() != size {
            buf.resize(size, 0);
        } else {
            buf.fill(0);
        }
        buf
    } else {
        vec![0u8; size]
    }
}

pub(super) fn return_scratch_buffer(st: &mut WorkerState, buf: Vec<u8>) {
    st.scratch_pool.push(buf);
}
