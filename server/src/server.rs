use crate::persistence::{Channel, FileDatabase};
use anyhow::{bail, Context, Error};
use dropshot::{
    endpoint, ApiDescription, ConfigDropshot, ConfigLogging, ConfigLoggingLevel, HttpError,
    HttpResponseOk, HttpServerStarter, RequestContext,
};
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::{Arc, Mutex};

pub async fn cancellable_server(database: Arc<Mutex<FileDatabase>>) -> Result<(), Error> {
    let log = ConfigLogging::StderrTerminal {
        level: ConfigLoggingLevel::Debug,
    }
    .to_logger("russ")
    .context("configuring dropshot logger")?;

    let mut api = ApiDescription::new();
    api.register(get_channels).unwrap();

    let server = HttpServerStarter::new(
        &ConfigDropshot {
            bind_address: "127.0.0.1:31981".parse().unwrap(),
            request_body_max_bytes: 10240,
        },
        api,
        RussContext { database },
        &log,
    )
    .map_err(|error| format!("failed to start server: {}", error))
    .expect("starting dropshot server")
    .start();

    if let Err(s) = server.await {
        bail!("error running server: {}", s);
    }

    // TODO I'm not smart enough to figure out how to actually call close on the server instance.
    // Awaiting the server seems to move and consume it, so I have no idea how you could actually
    // call close. None of the examples close the server, either, yet none of them panic like my
    // version does. Figure this out, since now this program will always panic on exit, and that's
    // garbage.
    //match server.await {
    //    Err(s) => bail!("error running server: {}", s),
    //    Ok(_) => server
    //        .close()
    //        .await
    //        .map_err(|error| format!("failed gracefully stopping server: {}", error)),
    //};

    Ok(())
}

struct RussContext {
    database: Arc<Mutex<FileDatabase>>,
}

#[derive(Serialize, JsonSchema)]
struct ChannelsList {
    channels: Vec<Channel>,
}

#[endpoint {
    method = GET,
    path = "/channels",
}]
async fn get_channels(
    rqctx: Arc<RequestContext<RussContext>>,
) -> Result<HttpResponseOk<ChannelsList>, HttpError> {
    match rqctx.context().database.lock() {
        Err(e) => Err(HttpError::for_internal_error(format!(
            "locking database: {}",
            e
        ))),
        Ok(mut d) => match d.get_channels() {
            Err(e) => Err(HttpError::for_internal_error(format!(
                "getting channels: {}",
                e
            ))),
            Ok(channels) => Ok(HttpResponseOk(ChannelsList { channels })),
        },
    }
}
