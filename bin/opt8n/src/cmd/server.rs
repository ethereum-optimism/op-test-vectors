use anvil::cmd::NodeArgs;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use clap::Parser;
use color_eyre::eyre::eyre;
use color_eyre::owo_colors::OwoColorize;
use futures::StreamExt;
use http_body_util::BodyExt;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::opt8n::{Opt8n, Opt8nArgs};

#[derive(Parser, Clone, Debug)]
pub struct ServerArgs {
    #[command(flatten)]
    pub opt8n_args: Opt8nArgs,
    #[command(flatten)]
    pub node_args: NodeArgs,
}

impl ServerArgs {
    pub async fn run(&self) -> color_eyre::Result<()> {
        let opt8n =
            Opt8n::new(Some(self.node_args.clone()), self.opt8n_args.output.clone()).await?;

        let opt8n = Arc::new(Mutex::new(opt8n));

        let (dump_tx, mut dump_rx) = tokio::sync::mpsc::channel::<()>(1);

        let dump_fixture_router = axum::Router::new()
            .route("/dump_fixture", axum::routing::post(dump_execution_fixture))
            .with_state((opt8n.clone(), dump_tx));

        let router = axum::Router::new()
            .route("/mine_prestate", axum::routing::post(mine_prestate))
            .fallback(fallback_handler)
            .with_state(opt8n);

        let router = dump_fixture_router.merge(router);

        let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        let server = axum::serve(listener, router.into_make_service());
        let _ = println!("Opt8n server listening on: {:#?}", local_addr).green();

        tokio::select! {
               err = server => {
                 todo!("Handle server error: {:#?}", err);
               }

               _ = dump_rx.recv() => {
                let _ = println!("Exuction fixture dumped to: {:#?}", self.opt8n_args.output).green();

               }
        }

        Ok(())
    }
}

async fn dump_execution_fixture(
    State((opt8n, dump_tx)): State<(Arc<Mutex<Opt8n>>, Sender<()>)>,
) -> Result<(), ServerError> {
    mine_block(opt8n).await?;
    dump_tx.send(()).await.map_err(ServerError::SendError)?;

    Ok(())
}

async fn mine_prestate(State(opt8n): State<Arc<Mutex<Opt8n>>>) -> Result<(), ServerError> {
    mine_block(opt8n.clone()).await?;
    Ok(())
}

async fn mine_block(opt8n: Arc<Mutex<Opt8n>>) -> Result<(), ServerError> {
    let mut opt8n = opt8n.lock().await;

    let mut new_blocks = opt8n.eth_api.backend.new_block_notifications();

    opt8n.mine_block().await;

    let block = new_blocks
        .next()
        .await
        .ok_or(eyre!("No new block"))
        .map_err(ServerError::Opt8nError)?;
    if let Some(block) = opt8n.eth_api.backend.get_block_by_hash(block.hash) {
        opt8n
            .generate_execution_fixture(block)
            .await
            .map_err(ServerError::Opt8nError)?;
    }

    Ok(())
}

async fn fallback_handler(
    State(opt8n): State<Arc<Mutex<Opt8n>>>,
    req: Request<Body>,
) -> Result<(), ServerError> {
    let anvil_endpoint = opt8n.lock().await.node_handle.http_endpoint();
    proxy_to_anvil(req, anvil_endpoint).await?;
    Ok(())
}

pub async fn proxy_to_anvil(
    req: Request<Body>,
    anvil_endpoint: String,
) -> Result<Response<Body>, ServerError> {
    let http_client = reqwest::Client::new();

    let (headers, body) = req.into_parts();
    let body = body
        .collect()
        .await
        .map_err(ServerError::AxumError)?
        .to_bytes();

    let axum_req: Request<Bytes> = Request::from_parts(headers, body);
    let mut req = reqwest::Request::try_from(axum_req).expect("TODO: handle error");
    req.url_mut().set_fragment(Some(&anvil_endpoint));

    let res: Response<reqwest::Body> = http_client
        .execute(req)
        .await
        .expect("TODO: handle error ")
        .into();

    let (headers, body) = res.into_parts();

    let body = body
        .collect()
        .await
        .map_err(ServerError::ReqwestError)?
        .to_bytes()
        .into();

    Ok(Response::from_parts(headers, body))
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Opt8n error: {0}")]
    Opt8nError(color_eyre::Report),
    #[error("Axum error: {0}")]
    AxumError(axum::Error),
    #[error("Reqwest error: {0}")]
    ReqwestError(reqwest::Error),
    #[error("Senderror: {0}")]
    SendError(tokio::sync::mpsc::error::SendError<()>),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let message = match self {
            ServerError::Opt8nError(err) => err.to_string(),
            ServerError::ReqwestError(err) => err.to_string(),
            ServerError::AxumError(err) => err.to_string(),
            ServerError::SendError(err) => err.to_string(),
        };

        let body = Body::from(message);

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
