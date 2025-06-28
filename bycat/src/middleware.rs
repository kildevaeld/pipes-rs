use crate::Work;

pub trait Middleware<C, B, H> {
    type Work: Work<C, B>;

    fn wrap(&self, handle: H) -> Self::Work;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Passthrough;

impl<B, C, H> Middleware<C, B, H> for Passthrough
where
    H: Work<C, B>,
{
    type Work = H;
    fn wrap(&self, handle: H) -> Self::Work {
        handle
    }
}
