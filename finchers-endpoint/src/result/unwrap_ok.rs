use finchers_core::endpoint::{Context, Endpoint};
use finchers_core::task::{self, Task};
use finchers_core::{Error, HttpError, PollResult};

pub fn new<E, T, R>(endpoint: E) -> UnwrapOk<E>
where
    E: Endpoint<Output = Result<T, R>>,
    R: HttpError,
{
    UnwrapOk { endpoint }
}

#[derive(Copy, Clone, Debug)]
pub struct UnwrapOk<E> {
    endpoint: E,
}

impl<E, T, R> Endpoint for UnwrapOk<E>
where
    E: Endpoint<Output = Result<T, R>>,
    R: HttpError,
{
    type Output = T;
    type Task = UnwrapOkTask<E::Task>;

    fn apply(&self, cx: &mut Context) -> Option<Self::Task> {
        Some(UnwrapOkTask {
            task: self.endpoint.apply(cx)?,
        })
    }
}

#[derive(Debug)]
pub struct UnwrapOkTask<T> {
    task: T,
}

impl<T, U, E> Task for UnwrapOkTask<T>
where
    T: Task<Output = Result<U, E>> + Send,
    E: HttpError,
{
    type Output = U;

    fn poll_task(&mut self, cx: &mut task::Context) -> PollResult<Self::Output, Error> {
        let res: Result<U, E> = poll_result!(self.task.poll_task(cx));
        res.map_err(Into::into).into()
    }
}
