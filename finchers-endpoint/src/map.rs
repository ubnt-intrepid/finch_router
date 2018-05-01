use finchers_core::endpoint::{Context, Endpoint, IntoEndpoint};
use finchers_core::task::{self, Task};
use finchers_core::{Error, PollResult};

pub fn new<E, F, T>(endpoint: E, f: F) -> Map<E::Endpoint, F>
where
    E: IntoEndpoint,
    F: FnOnce(E::Output) -> T + Clone + Send,
{
    Map {
        endpoint: endpoint.into_endpoint(),
        f,
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Map<E, F> {
    endpoint: E,
    f: F,
}

impl<E, F, T> Endpoint for Map<E, F>
where
    E: Endpoint,
    F: FnOnce(E::Output) -> T + Clone + Send,
{
    type Output = F::Output;
    type Task = MapTask<E::Task, F>;

    fn apply(&self, cx: &mut Context) -> Option<Self::Task> {
        Some(MapTask {
            task: self.endpoint.apply(cx)?,
            f: Some(self.f.clone()),
        })
    }
}

#[derive(Debug)]
pub struct MapTask<T, F> {
    task: T,
    f: Option<F>,
}

impl<T, F, U> Task for MapTask<T, F>
where
    T: Task + Send,
    F: FnOnce(T::Output) -> U + Send,
{
    type Output = U;

    fn poll_task(&mut self, cx: &mut task::Context) -> PollResult<Self::Output, Error> {
        self.task.poll_task(cx).map_ok(|item| {
            let f = self.f.take().expect("cannot resolve twice");
            cx.input().enter_scope(|| f(item))
        })
    }
}
