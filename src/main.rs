use std::{net::IpAddr, sync::Arc, time::Duration};

use axum::{extract::State, routing::get};
use prometheus::{Encoder, IntGauge, Registry, TextEncoder};
use reqwest::Client;
use tokio::{net::TcpListener, time::MissedTickBehavior};

const LINK: &str = "https://discord.com/api/v10/invites/minecraft?with_counts=true";

#[tokio::main]
async fn main() {
    let client = Client::new();
    let members = IntGauge::new("members", "How many total members there are").unwrap();
    let presences = IntGauge::new("presences", "How many members are online").unwrap();
    let boosts = IntGauge::new("boosts", "How many boosts the server has").unwrap();

    let reg = Registry::new();
    reg.register(Box::new(members.clone())).unwrap();
    reg.register(Box::new(presences.clone())).unwrap();
    reg.register(Box::new(boosts.clone())).unwrap();

    let state = Arc::new(reg);
    tokio::spawn(event_loop(
        Gauges {
            members,
            presences,
            boosts,
        },
        client,
    ));
    let app = axum::Router::new()
        .route("/metrics", get(metrics))
        .with_state(state);
    let tcp = TcpListener::bind((IpAddr::from([0, 0, 0, 0]), 9000))
        .await
        .unwrap();
    println!("Starting server on {}", tcp.local_addr().unwrap());
    axum::serve(tcp, app).await.unwrap();
}

pub async fn metrics(State(reg): State<Arc<Registry>>) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(8192);
    let encoder = TextEncoder::new();
    let metric_families = reg.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    buffer
}

pub async fn event_loop(gauges: Gauges, client: Client) {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let req = match client.get(LINK).send().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("req error: {e:?}");
                continue;
            }
        };
        let info: InviteInfo = match req.json().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("resp error: {e:?}");
                continue;
            }
        };
        println!("Got server data {info:?}");
        gauges.update(info);
    }
}

#[derive(Debug)]
pub struct Gauges {
    members: IntGauge,
    presences: IntGauge,
    boosts: IntGauge,
}

impl Gauges {
    pub fn update(&self, info: InviteInfo) {
        self.members.set(info.approximate_member_count);
        self.presences.set(info.approximate_presence_count);
        self.boosts.set(info.guild.premium_subscription_count);
    }
}

#[derive(Copy, Clone, Debug, serde::Deserialize)]
pub struct InviteInfo {
    guild: Guild,
    approximate_member_count: i64,
    approximate_presence_count: i64,
}

#[derive(Copy, Clone, Debug, serde::Deserialize)]
pub struct Guild {
    premium_subscription_count: i64,
}
