use std::str::FromStr;
use std::sync::Arc;

use alloy::signers::local::PrivateKeySigner;
use hyperliquid_rust_sdk::{
    BaseUrl, ClientLimit, ClientOrder, ClientOrderRequest, ExchangeClient, ExchangeResponseStatus,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderTif {
    Gtc,
    Ioc,
    Alo,
}

impl OrderTif {
    fn as_str(&self) -> &'static str {
        match self {
            OrderTif::Gtc => "Gtc",
            OrderTif::Ioc => "Ioc",
            OrderTif::Alo => "Alo",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub asset: String,
    pub side: OrderSide,
    pub size: String,
    pub limit_px: String,
    pub tif: OrderTif,
    pub reduce_only: bool,
    pub client_tag: String,
    pub cloid: Option<Uuid>,
}

impl OrderIntent {
    pub fn describe(&self) -> String {
        format!("{} {} {}", self.client_tag, self.asset, self.limit_px)
    }
}

#[derive(Clone)]
pub struct OrderRouter {
    client: Arc<ExchangeClient>,
    wallet_address: alloy::primitives::Address,
}

impl OrderRouter {
    pub async fn new(base_url: BaseUrl, signer_hex: &str) -> AppResult<Self> {
        let signer = PrivateKeySigner::from_str(signer_hex)
            .map_err(|e| AppError::Config(format!("invalid signer key: {e}")))?;
        let wallet_address = signer.address();
        let client = ExchangeClient::new(None, signer, Some(base_url), None, None)
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;
        Ok(Self {
            client: Arc::new(client),
            wallet_address,
        })
    }

    pub fn wallet_address(&self) -> alloy::primitives::Address {
        self.wallet_address
    }

    #[instrument(skip(self))]
    pub async fn submit(&self, intent: OrderIntent) -> AppResult<String> {
        let cloid = intent.cloid.unwrap_or_else(Uuid::new_v4);
        info!(
            asset = %intent.asset,
            side = ?intent.side,
            tif = intent.tif.as_str(),
            reduce_only = intent.reduce_only,
            cloid = %cloid,
            "submitting order"
        );

        let limit_px = intent
            .limit_px
            .parse::<f64>()
            .map_err(|e| AppError::Config(format!("invalid limit_px: {e}")))?;
        let sz = intent
            .size
            .parse::<f64>()
            .map_err(|e| AppError::Config(format!("invalid size: {e}")))?;

        let request = ClientOrderRequest {
            asset: intent.asset.clone(),
            is_buy: matches!(intent.side, OrderSide::Buy),
            reduce_only: intent.reduce_only,
            limit_px,
            sz,
            order_type: ClientOrder::Limit(ClientLimit {
                tif: intent.tif.as_str().to_string(),
            }),
            cloid: Some(cloid),
        };

        let response = self
            .client
            .order(request, None)
            .await
            .map_err(|e| AppError::Exchange(e.to_string()))?;

        match response {
            ExchangeResponseStatus::Ok(_) => Ok(cloid.to_string()),
            ExchangeResponseStatus::Err(err) => Err(AppError::Exchange(err.to_string())),
        }
    }
}
