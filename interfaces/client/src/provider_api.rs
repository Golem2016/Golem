use std::sync::Arc;
use futures::Future;

use crate::client::ApiClient;
use crate::error::Error;
use crate::market::{AgreementProposal, Offer, Proposal, ProviderEvent};



pub struct ProviderApiClient {
    //Client::default()
    client: Arc<ApiClient>,
}

//impl {
//    pub fn new(configuration: Rc<configuration::Configuration<C>>) -> ProviderApiClient<C> {
//        ProviderApiClient {
//            configuration: configuration,
//        }
//
//    }
//}

//pub trait ProviderApi {
//
//    /// Publish Provider’s service capabilities (Offer) on the market to declare an
//    /// interest in Demands meeting specified criteria.
//    fn subscribe(&self, offer: Offer) -> Box<Future<Item = String, Error = Error>>;
//
//    /// Stop subscription by invalidating a previously published Offer.
//    fn unsubscribe(&self, subscription_id: &str) -> Box<Future<Item = (), Error = Error>>;
//
//    /// Get events which have arrived from the market in response to the Offer
//    /// published by the Provider via  [subscribe](self::subscribe).
//    /// Returns collection of [ProviderEvents](ProviderEvent) or timeout.
//    fn collect(
//        &self,
//        subscription_id: &str,
//        timeout: f32,
//        max_events: i64,
//    ) -> Box<Future<Item = Vec<ProviderEvent>, Error = Error>>;
//
//    /// TODO
//    fn create_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//        proposal: Proposal,
//    ) -> Box<Future<Item = String, Error = Error>>;
//
//    /// TODO
//    fn get_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//    ) -> Box<Future<Item = AgreementProposal, Error = Error>>;
//
//    /// TODO
//    fn reject_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//    ) -> Box<Future<Item = (), Error = Error>>;
//
//    /// Confirms the Agreement received from the Requestor.
//    /// Mutually exclusive with [reject_agreement](self::reject_agreement).
//    fn approve_agreement(&self, agreement_id: &str) -> Box<Future<Item = (), Error = Error>>;
//
//    /// Rejects the Agreement received from the Requestor.
//    /// Mutually exclusive with [approve_agreement](self::approve_agreement).
//    fn reject_agreement(&self, agreement_id: &str) -> Box<Future<Item = (), Error = Error>>;
//}




//
//impl ProviderApi for ProviderApiClient {
//
//    fn subscribe(&self, offer: Offer) -> Box<Future<Item = String, Error = Error>> {
//        __internal_request::Request::new(hyper::Method::Post, "/offers".to_string())
//            .with_body_param(offer)
//            .execute(self.configuration.borrow())
//    }
//
//    fn unsubscribe(&self, subscription_id: &str) -> Box<Future<Item = (), Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Delete,
//            "/offers/{subscriptionId}".to_string(),
//        )
//            .with_path_param("subscriptionId".to_string(), subscription_id.to_string())
//            .returns_nothing()
//            .execute(self.configuration.borrow())
//    }
//
//    fn collect(
//        &self,
//        subscription_id: &str,
//        timeout: f32,
//        max_events: i64,
//    ) -> Box<Future<Item = Vec<ProviderEvent>, Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Get,
//            "/offers/{subscriptionId}/events".to_string(),
//        )
//        .with_query_param("timeout".to_string(), timeout.to_string())
//        .with_query_param("maxEvents".to_string(), max_events.to_string())
//        .with_path_param("subscriptionId".to_string(), subscription_id.to_string())
//        .execute(self.configuration.borrow())
//    }
//
//    fn create_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//        proposal: Proposal,
//    ) -> Box<Future<Item = String, Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Post,
//            "/offers/{subscriptionId}/proposals/{proposalId}/offer".to_string(),
//        )
//        .with_path_param("subscriptionId".to_string(), subscription_id.to_string())
//        .with_path_param("proposalId".to_string(), proposal_id.to_string())
//        .with_body_param(proposal)
//        .execute(self.configuration.borrow())
//    }
//
//    fn get_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//    ) -> Box<Future<Item = AgreementProposal, Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Get,
//            "/offers/{subscriptionId}/proposals/{proposalId}".to_string(),
//        )
//        .with_path_param("subscriptionId".to_string(), subscription_id.to_string())
//        .with_path_param("proposalId".to_string(), proposal_id.to_string())
//        .execute(self.configuration.borrow())
//    }
//
//    fn reject_proposal(
//        &self,
//        subscription_id: &str,
//        proposal_id: &str,
//    ) -> Box<Future<Item = (), Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Delete,
//            "/offers/{subscriptionId}/proposals/{proposalId}".to_string(),
//        )
//            .with_path_param("subscriptionId".to_string(), subscription_id.to_string())
//            .with_path_param("proposalId".to_string(), proposal_id.to_string())
//            .returns_nothing()
//            .execute(self.configuration.borrow())
//    }
//
//    fn approve_agreement(&self, agreement_id: &str) -> Box<Future<Item = (), Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Post,
//            "/agreements/{agreementId}/approve".to_string(),
//        )
//            .with_path_param("agreementId".to_string(), agreement_id.to_string())
//            .returns_nothing()
//            .execute(self.configuration.borrow())
//    }
//
//    fn reject_agreement(&self, agreement_id: &str) -> Box<Future<Item = (), Error = Error>> {
//        __internal_request::Request::new(
//            hyper::Method::Post,
//            "/agreements/{agreementId}/reject".to_string(),
//        )
//        .with_path_param("agreementId".to_string(), agreement_id.to_string())
//        .returns_nothing()
//        .execute(self.configuration.borrow())
//    }
//
//}
