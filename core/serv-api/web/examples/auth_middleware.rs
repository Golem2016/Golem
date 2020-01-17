use actix_web::http::header;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};

use awc::Client;
use futures::TryFutureExt;
use structopt::StructOpt;

use ya_core_model::identity as idm;
use ya_persistence::executor::DbExecutor;
use ya_service_api::constants::{YAGNA_BUS_ADDR, YAGNA_HTTP_ADDR, YAGNA_HTTP_ADDR_STR};
use ya_service_api_web::middleware::auth;
use ya_service_bus::RpcEndpoint;

async fn server() -> anyhow::Result<()> {
    let db = DbExecutor::new(":memory:")?;
    ya_sb_router::bind_router(*YAGNA_BUS_ADDR).await?;
    ya_identity::service::activate(&db).await?;

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(auth::Auth::default())
            .service(web::resource("/").route(web::get().to(|req: HttpRequest| {
                let body = format!("{:?}", req);
                HttpResponse::Ok().body(body)
            })))
    })
    .bind(*YAGNA_HTTP_ADDR)?
    .run()
    .await?;

    Ok(())
}

#[derive(StructOpt)]
enum ClientCommand {
    CreateKey { name: String },
    Request { key: String },
}

#[derive(StructOpt)]
enum Command {
    Server,
    Client(ClientCommand),
}

fn map_err<E: std::fmt::Debug>(e: E) -> anyhow::Error {
    anyhow::Error::msg(format!("{:?}", e))
}

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    match Command::from_args() {
        Command::Server => server().await?,
        Command::Client(cmd) => {
            use ya_core_model::appkey as model;
            use ya_service_bus::typed as bus;

            match cmd {
                ClientCommand::CreateKey { name } => {
                    let identity = bus::private_service(idm::IDENTITY_SERVICE_ID)
                        .send(idm::Get::ByDefault)
                        .await
                        .map_err(map_err)?
                        .map_err(map_err)?
                        .ok_or(anyhow::Error::msg("Identity not found"))?
                        .node_id;

                    let create = model::Create {
                        name,
                        role: model::DEFAULT_ROLE.to_string(),
                        identity,
                    };

                    let app_key = bus::private_service(model::APP_KEY_SERVICE_ID)
                        .send(create)
                        .await
                        .map_err(map_err)?
                        .map_err(map_err)?;

                    println!("{:?}", app_key);
                }
                ClientCommand::Request { key } => {
                    let mut resp = Client::default()
                        .get(format!("http://{}", *YAGNA_HTTP_ADDR_STR))
                        .header(header::AUTHORIZATION, key)
                        .send()
                        .map_err(map_err)
                        .await?;

                    let status = resp.status();
                    let body = resp.body().map_err(map_err).await?.to_vec();

                    println!("{}\n{}", status, String::from_utf8(body)?);
                }
            }
        }
    }

    Ok(())
}
