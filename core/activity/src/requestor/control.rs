use crate::common::{generate_id, PathActivity, QueryTimeout, QueryTimeoutMaxCount};
use crate::dao::{ActivityDao, ActivityStateDao, AgreementDao};
use crate::error::Error;
use crate::requestor::{get_agreement, missing_activity_err, provider_activity_uri};
use crate::ACTIVITY_SERVICE_URI;
use actix_web::web;
use futures::lock::Mutex;
use futures::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use ya_core_model::activity::{CreateActivity, DestroyActivity, Exec, GetExecBatchResults};
use ya_model::activity::{ExeScriptCommand, ExeScriptCommandResult, ExeScriptRequest, State};
use ya_persistence::executor::DbExecutor;

pub fn web_scope(db: Arc<Mutex<DbExecutor>>) -> actix_web::Scope {
    let create = web::post().to(impl_restful_handler!(create_activity, path, query));
    let delete = web::delete().to(impl_restful_handler!(destroy_activity, path, query));
    let exec = web::post().to(impl_restful_handler!(exec, path, query, body));
    let batch = web::get().to(impl_restful_handler!(get_batch_results, path, query));

    web::scope(&ACTIVITY_SERVICE_URI)
        .data(db)
        .service(web::resource("/activity").route(create))
        .service(web::resource("/activity/{activity_id}").route(delete))
        .service(web::resource("/activity/{activity_id}/exec").route(exec))
        .service(web::resource("/activity/{activity_id}/exec/{batch_id}").route(batch))
}

/// Creates new Activity based on given Agreement.
async fn create_activity(
    db: web::Data<Arc<Mutex<DbExecutor>>>,
    query: web::Query<QueryTimeout>,
    body: web::Json<CreateActivity>,
) -> Result<String, Error> {
    let conn = db_conn!(db)?;
    let agreement_id = body.agreement_id.clone();
    let agreement = AgreementDao::new(&conn).get(&agreement_id)?;

    let uri = provider_activity_uri(&agreement.offer_node_id);
    let activity_id = gsb_send!(body.into_inner(), &uri, query.timeout)?;
    ActivityDao::new(&conn)
        .create(&activity_id, &agreement_id)
        .map_err(Error::from)?;

    Ok(activity_id)
}

/// Destroys given Activity.
async fn destroy_activity(
    db: web::Data<Arc<Mutex<DbExecutor>>>,
    path: web::Path<PathActivity>,
    query: web::Query<QueryTimeout>,
) -> Result<(), Error> {
    let conn = db_conn!(db)?;
    missing_activity_err(&conn, &path.activity_id)?;

    let agreement = get_agreement(&conn, &path.activity_id)?;
    let uri = provider_activity_uri(&agreement.offer_node_id);
    let msg = DestroyActivity {
        activity_id: path.activity_id.to_string(),
        agreement_id: agreement.natural_id,
        timeout: query.timeout.clone(),
    };

    let _ = gsb_send!(msg, &uri, query.timeout)?;
    ActivityStateDao::new(&db_conn!(db)?)
        .set(&path.activity_id, State::Terminated, None, None)
        .map_err(Error::from)?;

    Ok(())
}

/// Executes an ExeScript batch within a given Activity.
async fn exec(
    db: web::Data<Arc<Mutex<DbExecutor>>>,
    path: web::Path<PathActivity>,
    query: web::Query<QueryTimeout>,
    body: web::Json<ExeScriptRequest>,
) -> Result<String, Error> {
    let conn = db_conn!(db)?;
    missing_activity_err(&conn, &path.activity_id)?;

    let commands: Vec<ExeScriptCommand> =
        serde_json::from_str(&body.text).map_err(|e| Error::BadRequest(format!("{:?}", e)))?;
    let agreement = get_agreement(&conn, &path.activity_id)?;
    let uri = provider_activity_uri(&agreement.offer_node_id);
    let batch_id = generate_id();
    let msg = Exec {
        activity_id: path.activity_id.clone(),
        batch_id: batch_id.clone(),
        exe_script: commands,
        timeout: query.timeout.clone(),
    };

    gsb_send!(msg, &uri, query.timeout)?;
    Ok(batch_id)
}

/// Queries for ExeScript batch results.
async fn get_batch_results(
    db: web::Data<Arc<Mutex<DbExecutor>>>,
    path: web::Path<PathActivityBatch>,
    query: web::Query<QueryTimeoutMaxCount>,
) -> Result<Vec<ExeScriptCommandResult>, Error> {
    let conn = db_conn!(db)?;
    missing_activity_err(&conn, &path.activity_id)?;

    let agreement = get_agreement(&conn, &path.activity_id)?;
    let uri = provider_activity_uri(&agreement.offer_node_id);
    let msg = GetExecBatchResults {
        activity_id: path.activity_id.to_string(),
        batch_id: path.batch_id.to_string(),
        timeout: query.timeout.clone(),
    };

    gsb_send!(msg, &uri, query.timeout)
}

#[derive(Deserialize)]
struct PathActivityBatch {
    activity_id: String,
    batch_id: String,
}
