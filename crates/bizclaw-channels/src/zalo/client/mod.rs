//! Zalo client sub-modules — auth, messaging, session, crypto, WebSocket listener, bot.
pub mod auth;
pub mod bank_helpers;
/// Zalo OA Bot Platform client (bot.zapps.me official API).
pub mod bot;
pub mod business;
pub mod crypto;
pub mod friends;
pub mod groups;
pub mod listener;
pub mod messaging;
pub mod models;
pub mod session;
