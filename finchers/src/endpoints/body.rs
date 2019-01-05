//! Endpoints for parsing the message body.

use {
    crate::{
        endpoint::{
            ActionContext, //
            ApplyContext,
            Endpoint,
            EndpointAction,
            IsEndpoint,
            Preflight,
        },
        error::{BadRequest, Error, InternalServerError},
    },
    futures::Poll,
    http::Request,
    izanami_service::http::BufStream,
    mime::Mime,
    serde::de::DeserializeOwned,
    std::{cell::UnsafeCell, marker::PhantomData},
};

fn stolen_payload() -> Error {
    InternalServerError::from(
        "The instance of request body has already been stolen by another endpoint.",
    )
    .into()
}

fn content_type<T>(request: &Request<T>) -> crate::error::Result<Option<Mime>> {
    if let Some(h) = request.headers().get(http::header::CONTENT_TYPE) {
        let mime = h
            .to_str()
            .map_err(BadRequest::from)?
            .parse()
            .map_err(BadRequest::from)?;
        Ok(Some(mime))
    } else {
        Ok(None)
    }
}

/// Creates an endpoint which takes the instance of request body from the context.
///
/// If the instance of request body has already been stolen by another endpoint,
/// it will return an error.
#[inline]
pub fn raw() -> Raw {
    Raw(())
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct Raw(());

mod raw {
    use super::*;

    impl IsEndpoint for Raw {}

    impl<Bd> Endpoint<Bd> for Raw {
        type Output = (Bd,);
        type Error = Error;
        type Action = RawAction;

        fn action(&self) -> Self::Action {
            RawAction {
                _anchor: PhantomData,
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct RawAction {
        _anchor: PhantomData<UnsafeCell<()>>,
    }

    impl<Bd> EndpointAction<Bd> for RawAction {
        type Output = (Bd,);
        type Error = Error;

        fn poll_action(
            &mut self,
            cx: &mut ActionContext<'_, Bd>,
        ) -> Poll<Self::Output, Self::Error> {
            cx.body()
                .take()
                .map(|x| (x,).into())
                .ok_or_else(stolen_payload)
        }
    }
}

/// Creates an endpoint which receives all of request body.
///
/// If the instance of `BufStream` has already been stolen by another endpoint, it will
/// return an error.
#[inline]
pub fn receive_all() -> ReceiveAll {
    ReceiveAll(())
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct ReceiveAll(());

mod receive_all {
    use super::*;
    use bytes::Buf;

    impl IsEndpoint for ReceiveAll {}

    impl<Bd> Endpoint<Bd> for ReceiveAll
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        type Output = (Vec<u8>,);
        type Error = Error;
        type Action = ReceiveAllAction<Bd>;

        fn action(&self) -> Self::Action {
            new_action()
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct ReceiveAllAction<Bd> {
        state: State<Bd>,
    }

    #[allow(missing_debug_implementations)]
    enum State<Bd> {
        Start,
        Receiving(Bd, Vec<u8>),
    }

    impl<Bd> EndpointAction<Bd> for ReceiveAllAction<Bd>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        type Output = (Vec<u8>,);
        type Error = Error;

        fn poll_action(
            &mut self,
            cx: &mut ActionContext<'_, Bd>,
        ) -> Poll<Self::Output, Self::Error> {
            loop {
                self.state = match self.state {
                    State::Start => {
                        let payload = cx.body().take().ok_or_else(super::stolen_payload)?;
                        State::Receiving(payload, Vec::new())
                    }
                    State::Receiving(ref mut body, ref mut buf) => {
                        while let Some(data) = futures::try_ready!(body
                            .poll_buf()
                            .map_err(|e| failure::Error::from_boxed_compat(e.into())))
                        {
                            buf.extend_from_slice(data.bytes());
                        }
                        let buf = std::mem::replace(buf, Vec::new());
                        return Ok((buf,).into());
                    }
                };
            }
        }
    }

    pub(super) fn new_action<Bd>() -> ReceiveAllAction<Bd>
    where
        Bd: BufStream,
    {
        ReceiveAllAction {
            state: State::Start,
        }
    }
}

// ==== Text ====

/// Create an endpoint which parses a request body into `String`.
#[inline]
pub fn text() -> Text {
    Text {
        receive_all: receive_all(),
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct Text {
    receive_all: ReceiveAll,
}

mod text {
    use super::*;

    impl IsEndpoint for Text {}

    impl<Bd> Endpoint<Bd> for Text
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        type Output = (String,);
        type Error = Error;
        type Action = TextAction<Bd>;

        fn action(&self) -> Self::Action {
            TextAction {
                receive_all: self.receive_all.action(),
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct TextAction<Bd>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        receive_all: <ReceiveAll as Endpoint<Bd>>::Action,
    }

    impl<Bd> EndpointAction<Bd> for TextAction<Bd>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        type Output = (String,);
        type Error = Error;

        fn preflight(
            &mut self,
            cx: &mut ApplyContext<'_>,
        ) -> Result<Preflight<Self::Output>, Self::Error> {
            let x = self.receive_all.preflight(cx)?;
            debug_assert!(x.is_incomplete());
            drop(x);

            if let Some(param) = content_type(&*cx)?
                .as_ref()
                .and_then(|m| m.get_param("charset"))
            {
                if param != "utf-8" {
                    return Err(BadRequest::from("Only the UTF-8 charset is supported.").into());
                }
            }

            Ok(Preflight::Incomplete)
        }

        fn poll_action(
            &mut self,
            cx: &mut ActionContext<'_, Bd>,
        ) -> Poll<Self::Output, Self::Error> {
            let (data,) = futures::try_ready!(self.receive_all.poll_action(cx));
            String::from_utf8(data.to_vec())
                .map(|x| (x,).into())
                .map_err(BadRequest::from)
                .map_err(Into::into)
        }
    }
}

/// Create an endpoint which parses a request body into a JSON data.
#[inline]
pub fn json<T>() -> Json<T>
where
    T: DeserializeOwned,
{
    Json {
        receive_all: receive_all(),
        _marker: PhantomData,
    }
}

#[allow(missing_docs)]
pub struct Json<T> {
    receive_all: ReceiveAll,
    _marker: PhantomData<fn() -> T>,
}

mod json {
    use super::*;
    use std::fmt;

    impl<T> fmt::Debug for Json<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Json").finish()
        }
    }

    impl<T: DeserializeOwned> IsEndpoint for Json<T> {}

    impl<T, Bd> Endpoint<Bd> for Json<T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        T: DeserializeOwned,
    {
        type Output = (T,);
        type Error = Error;
        type Action = JsonAction<Bd, T>;

        fn action(&self) -> Self::Action {
            JsonAction {
                receive_all: self.receive_all.action(),
                _marker: PhantomData,
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct JsonAction<Bd, T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        receive_all: <ReceiveAll as Endpoint<Bd>>::Action,
        _marker: PhantomData<fn() -> T>,
    }

    impl<Bd, T> EndpointAction<Bd> for JsonAction<Bd, T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        T: DeserializeOwned,
    {
        type Output = (T,);
        type Error = Error;

        fn preflight(
            &mut self,
            cx: &mut ApplyContext<'_>,
        ) -> Result<Preflight<Self::Output>, Self::Error> {
            let x = self.receive_all.preflight(cx)?;
            debug_assert!(x.is_incomplete());
            drop(x);

            let mime = content_type(&*cx)? //
                .ok_or_else(|| BadRequest::from("missing content type"))?;
            if mime != mime::APPLICATION_JSON {
                return Err(BadRequest::from(
                    "The value of `Content-type` must be `application/json`.",
                )
                .into());
            }

            Ok(Preflight::Incomplete)
        }

        fn poll_action(
            &mut self,
            cx: &mut ActionContext<'_, Bd>,
        ) -> Poll<Self::Output, Self::Error> {
            let (data,) = futures::try_ready!(self.receive_all.poll_action(cx));
            serde_json::from_slice(&*data)
                .map(|x| (x,).into())
                .map_err(BadRequest::from)
                .map_err(Into::into)
        }
    }
}

// ==== UrlEncoded ====

/// Create an endpoint which parses an urlencoded data.
#[inline]
pub fn urlencoded<T>() -> Urlencoded<T>
where
    T: DeserializeOwned,
{
    Urlencoded {
        receive_all: receive_all(),
        _marker: PhantomData,
    }
}

#[allow(missing_docs)]
pub struct Urlencoded<T> {
    receive_all: ReceiveAll,
    _marker: PhantomData<fn() -> T>,
}

mod urlencoded {
    use super::*;
    use {failure::SyncFailure, std::fmt};

    impl<T> fmt::Debug for Urlencoded<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Json").finish()
        }
    }

    impl<T: DeserializeOwned> IsEndpoint for Urlencoded<T> {}

    impl<T, Bd> Endpoint<Bd> for Urlencoded<T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        T: DeserializeOwned,
    {
        type Output = (T,);
        type Error = Error;
        type Action = UrlencodedAction<Bd, T>;

        fn action(&self) -> Self::Action {
            UrlencodedAction {
                receive_all: self.receive_all.action(),
                _marker: PhantomData,
            }
        }
    }

    #[allow(missing_debug_implementations)]
    pub struct UrlencodedAction<Bd, T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    {
        receive_all: <ReceiveAll as Endpoint<Bd>>::Action,
        _marker: PhantomData<fn() -> T>,
    }

    impl<Bd, T> EndpointAction<Bd> for UrlencodedAction<Bd, T>
    where
        Bd: BufStream,
        Bd::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
        T: DeserializeOwned,
    {
        type Output = (T,);
        type Error = Error;

        fn preflight(
            &mut self,
            cx: &mut ApplyContext<'_>,
        ) -> Result<Preflight<Self::Output>, Self::Error> {
            let x = self.receive_all.preflight(cx)?;
            debug_assert!(x.is_incomplete());
            drop(x);

            let mime = content_type(&*cx)? //
                .ok_or_else(|| BadRequest::from("missing content type"))?;
            if mime != mime::APPLICATION_WWW_FORM_URLENCODED {
                return Err(BadRequest::from(
                    "The value of `Content-type` must be `application-x-www-form-urlencoded`.",
                )
                .into());
            }

            Ok(Preflight::Incomplete)
        }

        fn poll_action(
            &mut self,
            cx: &mut ActionContext<'_, Bd>,
        ) -> Poll<Self::Output, Self::Error> {
            let (data,) = futures::try_ready!(self.receive_all.poll_action(cx));
            let s = std::str::from_utf8(&*data).map_err(BadRequest::from)?;
            serde_qs::from_str(s)
                .map(|x| (x,).into())
                .map_err(|err| BadRequest::from(SyncFailure::new(err)).into())
        }
    }
}