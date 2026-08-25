use glib::{clone, ControlFlow};
use gtk::prelude::*;
use gtk::{Button, Orientation};
use pango::WrapMode;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::ApplicationSettings;
use crate::currencies::btc_chain;
use crate::currencies::eth_chain;
use crate::currencies::ltc_chain;
use crate::currencies::sol_chain;
use crate::currencies::tokens::Token;

pub fn transaction_view(app_settings: ApplicationSettings, token: Token) -> (gtk::Box, ApplicationSettings) {
    match token.chain.as_str() {
        "btc" => (btc_send_view(app_settings.clone()), app_settings),
        "sol" => (sol_send_view(app_settings.clone(), token), app_settings),
        "ltc" => (ltc_send_view(app_settings.clone()), app_settings),
        _ => (eth_send_view(app_settings.clone(), token), app_settings),
    }
}

fn btc_send_view(app_settings: ApplicationSettings) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let names: Vec<String> = app_settings
        .btc_wallets
        .iter()
        .map(|wallet| {
            wallet
                .wallet_name
                .clone()
                .unwrap_or_else(|| "Bitcoin".to_string())
        })
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let from_wallet = gtk::DropDown::from_strings(&name_refs);

    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recipient (bc1… / tb1…)")
        .hexpand(true)
        .build();
    let amount = gtk::Entry::builder()
        .placeholder_text("Amount (BTC)")
        .hexpand(true)
        .build();
    let fee = gtk::DropDown::from_strings(&["Low", "Medium", "High"]);
    fee.set_selected(1);

    let review = Button::builder().label("Review send").hexpand(true).build();
    review.add_css_class("standard_button");
    review.add_css_class("suggested-action");

    let error = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let confirm_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .css_classes(["receiver-box"])
        .build();
    let summary = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    let confirm = Button::builder().label("Confirm and broadcast").hexpand(true).build();
    confirm.add_css_class("standard_button");
    confirm.add_css_class("suggested-action");
    let cancel = Button::builder().label("Cancel").hexpand(true).build();
    cancel.add_css_class("standard_button");
    let btc_mainnet = !app_settings.btc_network.eq_ignore_ascii_case("testnet");
    let ack = gtk::CheckButton::with_label("I understand this spends real bitcoin.");
    ack.set_visible(btc_mainnet);
    if btc_mainnet {
        confirm.set_sensitive(false);
    }
    let confirm_gate = confirm.clone();
    ack.connect_toggled(move |cb| {
        confirm_gate.set_sensitive(cb.is_active());
    });
    confirm_box.append(&summary);
    confirm_box.append(&ack);
    confirm_box.append(&confirm);
    confirm_box.append(&cancel);

    let status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-standard"])
        .build();

    box_.append(&from_wallet);
    box_.append(&receive_address);
    box_.append(&amount);
    box_.append(&fee);
    box_.append(&review);
    box_.append(&error);
    box_.append(&confirm_box);
    box_.append(&status);

    let app_settings = Arc::new(Mutex::new(app_settings));
    let prepared = Arc::new(Mutex::new(None::<btc_chain::PreparedSend>));

    review.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] receive_address,
        #[weak] amount,
        #[weak] fee,
        #[weak] error,
        #[weak] confirm_box,
        #[weak] summary,
        #[weak] from_wallet,
        move |_| {
            error.set_visible(false);
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.btc_wallets.get(index) else {
                error.set_label("Select a Bitcoin account.");
                error.set_visible(true);
                return;
            };
            let Some(mnemonic) = wallet.mnemonic.clone() else {
                error.set_label("This account has no recovery phrase in memory. Unlock again.");
                error.set_visible(true);
                return;
            };
            let passphrase = wallet.password.clone().unwrap_or_default();
            let to = receive_address.text().to_string();
            let network = crate::currencies::btc_chain::parse_network(&settings.btc_network);
            if let Err(err) = btc_chain::validate_address(&to, network) {
                error.set_label(&format!("{err:?}").replace("ERROR: ", "").replace('\"', ""));
                error.set_visible(true);
                return;
            }
            let amount_sats = match btc_chain::btc_to_sats(&amount.text()) {
                Ok(value) if value > 0 => value,
                Ok(_) => {
                    error.set_label("Amount must be greater than 0.");
                    error.set_visible(true);
                    return;
                }
                Err(_) => {
                    error.set_label("Amount must be a BTC number, for example 0.001.");
                    error.set_visible(true);
                    return;
                }
            };
            let labels = ["Low", "Medium", "High"];
            let fee_label = labels
                .get(fee.selected() as usize)
                .copied()
                .unwrap_or("Medium")
                .to_string();
            let node = settings.btc_node.clone();
            let network_name = settings.btc_network.clone();
            let testnet = network_name.eq_ignore_ascii_case("testnet");
            drop(settings);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let tiers = btc_chain::fetch_fee_tiers(&node, &network_name);
                let fee_rate = btc_chain::fee_rate_from_tier(&tiers, &fee_label);
                let result = btc_chain::prepare_send(
                    &mnemonic,
                    &passphrase,
                    &network_name,
                    &node,
                    &to,
                    amount_sats,
                    fee_rate,
                );
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] error,
                    #[weak] confirm_box,
                    #[weak] summary,
                    #[strong] prepared,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(plan) => {
                                let prefix = if testnet {
                                    "Bitcoin testnet — coins have no mainnet value.\n\n"
                                } else {
                                    "MAINNET: this spends real bitcoin.\n\n"
                                };
                                summary.set_label(&format!("{}{}", prefix, plan.summary()));
                                *prepared.lock().unwrap() = Some(plan);
                                confirm_box.set_visible(true);
                            }
                            Err(_) => {
                                error.set_label("Could not build that transaction. Check amount, address, balance, or whether the Bitcoin node is reachable. Receive still works offline.");
                                error.set_visible(true);
                                confirm_box.set_visible(false);
                            }
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    cancel.connect_clicked(clone!(
        #[weak] confirm_box,
        #[strong] prepared,
        move |_| {
            *prepared.lock().unwrap() = None;
            confirm_box.set_visible(false);
        }
    ));

    confirm.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] error,
        #[weak] status,
        #[weak] confirm_box,
        #[weak] from_wallet,
        move |_| {
            let plan = match prepared.lock().unwrap().clone() {
                Some(plan) => plan,
                None => return,
            };
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.btc_wallets.get(index) else {
                return;
            };
            let Some(mnemonic) = wallet.mnemonic.clone() else {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let passphrase = wallet.password.clone().unwrap_or_default();
            let node = settings.btc_node.clone();
            let network_name = settings.btc_network.clone();
            drop(settings);

            error.set_visible(false);
            status.set_label("Broadcasting…");
            status.set_visible(true);
            confirm_box.set_visible(false);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = btc_chain::sign_and_broadcast(
                    &mnemonic,
                    &passphrase,
                    &network_name,
                    &node,
                    &plan.to,
                    plan.amount_sats,
                    plan.fee_rate_sat_vb,
                );
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] status,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(txid) => status.set_label(&format!("Sent. Transaction ID: {txid}")),
                            Err(_) => status.set_label(
                                "Broadcast failed. Node may be unreachable; the receive address is still valid offline.",
                            ),
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    box_
}

fn ltc_send_view(app_settings: ApplicationSettings) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let names: Vec<String> = app_settings
        .ltc_wallets
        .iter()
        .map(|wallet| {
            wallet
                .wallet_name
                .clone()
                .unwrap_or_else(|| "Litecoin".to_string())
        })
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let from_wallet = gtk::DropDown::from_strings(&name_refs);

    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recipient (ltc1… / tltc1…)")
        .hexpand(true)
        .build();
    let amount = gtk::Entry::builder()
        .placeholder_text("Amount (LTC)")
        .hexpand(true)
        .build();
    let fee = gtk::DropDown::from_strings(&["Low", "Medium", "High"]);
    fee.set_selected(1);

    let review = Button::builder().label("Review send").hexpand(true).build();
    review.add_css_class("standard_button");
    review.add_css_class("suggested-action");

    let error = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let confirm_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .css_classes(["receiver-box"])
        .build();
    let summary = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    let confirm = Button::builder().label("Confirm and broadcast").hexpand(true).build();
    confirm.add_css_class("standard_button");
    confirm.add_css_class("suggested-action");
    let cancel = Button::builder().label("Cancel").hexpand(true).build();
    cancel.add_css_class("standard_button");
    let ltc_mainnet = !app_settings.ltc_network.eq_ignore_ascii_case("testnet");
    let ack = gtk::CheckButton::with_label("I understand this spends real litecoin.");
    ack.set_visible(ltc_mainnet);
    if ltc_mainnet {
        confirm.set_sensitive(false);
    }
    let confirm_gate = confirm.clone();
    ack.connect_toggled(move |cb| {
        confirm_gate.set_sensitive(cb.is_active());
    });
    confirm_box.append(&summary);
    confirm_box.append(&ack);
    confirm_box.append(&confirm);
    confirm_box.append(&cancel);

    let status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-standard"])
        .build();

    box_.append(&from_wallet);
    box_.append(&receive_address);
    box_.append(&amount);
    box_.append(&fee);
    box_.append(&review);
    box_.append(&error);
    box_.append(&confirm_box);
    box_.append(&status);

    let app_settings = Arc::new(Mutex::new(app_settings));
    let prepared = Arc::new(Mutex::new(None::<ltc_chain::PreparedSend>));

    review.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] receive_address,
        #[weak] amount,
        #[weak] fee,
        #[weak] error,
        #[weak] confirm_box,
        #[weak] summary,
        #[weak] from_wallet,
        move |_| {
            error.set_visible(false);
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.ltc_wallets.get(index) else {
                error.set_label("Select a Litecoin account.");
                error.set_visible(true);
                return;
            };
            let Some(from) = wallet.address.clone() else {
                error.set_label("Wallet address is missing.");
                error.set_visible(true);
                return;
            };
            if wallet.private_key.is_none() {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let to = receive_address.text().to_string();
            let network = ltc_chain::parse_network(&settings.ltc_network);
            if let Err(err) = ltc_chain::validate_address(&to, network) {
                error.set_label(&format!("{err:?}").replace("ERROR: ", "").replace('\"', ""));
                error.set_visible(true);
                return;
            }
            let node = settings.ltc_node.clone();
            let network_name = settings.ltc_network.clone();
            let testnet = network_name.eq_ignore_ascii_case("testnet");
            drop(settings);

            let labels = ["Low", "Medium", "High"];
            let fee_label = labels
                .get(fee.selected() as usize)
                .copied()
                .unwrap_or("Medium")
                .to_string();
            let amount_text = amount.text().to_string();
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = ltc_chain::prepare_send(&from, &to, &amount_text, &node, &network_name, &fee_label);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] error,
                    #[weak] confirm_box,
                    #[weak] summary,
                    #[strong] prepared,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(plan) => {
                                let prefix = if testnet {
                                    "Litecoin testnet — coins have no mainnet value.\n\n"
                                } else {
                                    "MAINNET: this spends real litecoin.\n\n"
                                };
                                summary.set_label(&format!("{}{}", prefix, plan.summary()));
                                *prepared.lock().unwrap() = Some(plan);
                                confirm_box.set_visible(true);
                            }
                            Err(_) => {
                                error.set_label("Could not build that transaction. Check amount, address, balance, or whether the Litecoin node is reachable. Receive still works offline.");
                                error.set_visible(true);
                                confirm_box.set_visible(false);
                            }
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    cancel.connect_clicked(clone!(
        #[weak] confirm_box,
        #[strong] prepared,
        move |_| {
            *prepared.lock().unwrap() = None;
            confirm_box.set_visible(false);
        }
    ));

    confirm.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] error,
        #[weak] status,
        #[weak] confirm_box,
        #[weak] from_wallet,
        move |_| {
            let plan = match prepared.lock().unwrap().clone() {
                Some(plan) => plan,
                None => return,
            };
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.ltc_wallets.get(index) else {
                return;
            };
            let Some(private_key) = wallet.private_key.clone() else {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let node = settings.ltc_node.clone();
            let network_name = settings.ltc_network.clone();
            drop(settings);

            error.set_visible(false);
            status.set_label("Broadcasting…");
            status.set_visible(true);
            confirm_box.set_visible(false);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = ltc_chain::sign_and_broadcast(&private_key, &plan, &node, &network_name);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] status,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(txid) => status.set_label(&format!("Sent. Transaction ID: {txid}")),
                            Err(_) => status.set_label(
                                "Broadcast failed. Node may be unreachable; the receive address is still valid offline.",
                            ),
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    box_
}

fn eth_send_view(app_settings: ApplicationSettings, token: Token) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let names: Vec<String> = app_settings
        .eth_wallets
        .iter()
        .map(|wallet| {
            wallet
                .wallet_name
                .clone()
                .unwrap_or_else(|| "Ethereum".to_string())
        })
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let from_wallet = gtk::DropDown::from_strings(&name_refs);

    let symbol = token.symbol.clone();
    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recipient (0x…)")
        .hexpand(true)
        .build();
    let amount = gtk::Entry::builder()
        .placeholder_text(&format!("Amount ({symbol})"))
        .hexpand(true)
        .build();
    let fee = gtk::DropDown::from_strings(&["Low", "Medium", "High"]);
    fee.set_selected(1);

    let review = Button::builder().label("Review send").hexpand(true).build();
    review.add_css_class("standard_button");
    review.add_css_class("suggested-action");

    let error = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let confirm_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .css_classes(["receiver-box"])
        .build();
    let summary = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    let confirm = Button::builder().label("Confirm and broadcast").hexpand(true).build();
    confirm.add_css_class("standard_button");
    confirm.add_css_class("suggested-action");
    let cancel = Button::builder().label("Cancel").hexpand(true).build();
    cancel.add_css_class("standard_button");
    // Gate on "is this a known testnet", not "is this literally mainnet" — an unrecognized or
    // new network name (e.g. an L2) must default to showing the real-value warning, not hiding it.
    let eth_real_value = !eth_chain::is_testnet(eth_chain::parse_network(&app_settings.eth_network));
    let ack = gtk::CheckButton::with_label("I understand this spends real value.");
    ack.set_visible(eth_real_value);
    if eth_real_value {
        confirm.set_sensitive(false);
    }
    let confirm_gate = confirm.clone();
    ack.connect_toggled(move |cb| {
        confirm_gate.set_sensitive(cb.is_active());
    });
    confirm_box.append(&summary);
    confirm_box.append(&ack);
    confirm_box.append(&confirm);
    confirm_box.append(&cancel);

    let status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-standard"])
        .build();

    box_.append(&from_wallet);
    box_.append(&receive_address);
    box_.append(&amount);
    box_.append(&fee);
    box_.append(&review);
    box_.append(&error);
    box_.append(&confirm_box);
    box_.append(&status);

    let app_settings = Arc::new(Mutex::new(app_settings));
    let prepared = Arc::new(Mutex::new(None::<eth_chain::PreparedSend>));
    let token = Arc::new(token);

    review.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[strong] token,
        #[weak] receive_address,
        #[weak] amount,
        #[weak] fee,
        #[weak] error,
        #[weak] confirm_box,
        #[weak] summary,
        #[weak] from_wallet,
        move |_| {
            error.set_visible(false);
            confirm_box.set_visible(false);
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.eth_wallets.get(index) else {
                error.set_label("Select an Ethereum account.");
                error.set_visible(true);
                return;
            };
            let Some(from) = wallet.address.clone() else {
                error.set_label("Wallet address is missing.");
                error.set_visible(true);
                return;
            };
            if wallet.private_key.is_none() {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let to = receive_address.text().to_string();
            if let Err(_) = eth_chain::validate_address(&to) {
                error.set_label("Recipient must be a 0x Ethereum address. ENS is not supported yet.");
                error.set_visible(true);
                return;
            }
            let node = settings.eth_node.clone();
            let network_name = settings.eth_network.clone();
            let infura_key = settings.infura_key.clone();
            let testnet = network_name.eq_ignore_ascii_case("sepolia");
            drop(settings);

            let labels = ["Low", "Medium", "High"];
            let fee_label = labels
                .get(fee.selected() as usize)
                .copied()
                .unwrap_or("Medium")
                .to_string();
            let amount_text = amount.text().to_string();
            let token = (*token).clone();
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = eth_chain::prepare_send(
                    &from,
                    &to,
                    &amount_text,
                    &token,
                    &node,
                    &network_name,
                    &infura_key,
                    &fee_label,
                );
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] error,
                    #[weak] confirm_box,
                    #[weak] summary,
                    #[strong] prepared,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(plan) => {
                                let prefix = if testnet {
                                    "Sepolia testnet — coins have no mainnet value.\n\n"
                                } else {
                                    "MAINNET: this spends real ETH or tokens.\n\n"
                                };
                                summary.set_label(&format!("{}{}", prefix, plan.summary()));
                                *prepared.lock().unwrap() = Some(plan);
                                confirm_box.set_visible(true);
                            }
                            Err(_) => {
                                error.set_label("Could not build that transaction. Check amount, address, balance, or whether the Ethereum node is reachable. Receive still works offline.");
                                error.set_visible(true);
                                confirm_box.set_visible(false);
                            }
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    cancel.connect_clicked(clone!(
        #[weak] confirm_box,
        #[strong] prepared,
        move |_| {
            *prepared.lock().unwrap() = None;
            confirm_box.set_visible(false);
        }
    ));

    confirm.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] error,
        #[weak] status,
        #[weak] confirm_box,
        #[weak] from_wallet,
        move |_| {
            let plan = match prepared.lock().unwrap().clone() {
                Some(plan) => plan,
                None => return,
            };
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.eth_wallets.get(index) else {
                return;
            };
            let Some(private_key) = wallet.private_key.clone() else {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let node = settings.eth_node.clone();
            let network_name = settings.eth_network.clone();
            let infura_key = settings.infura_key.clone();
            drop(settings);

            error.set_visible(false);
            status.set_label("Broadcasting…");
            status.set_visible(true);
            confirm_box.set_visible(false);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = eth_chain::sign_and_broadcast(
                    &private_key,
                    &plan,
                    &node,
                    &network_name,
                    &infura_key,
                );
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] status,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(txid) => status.set_label(&format!("Sent. Transaction ID: {txid}")),
                            Err(_) => status.set_label(
                                "Broadcast failed. Node may be unreachable; the receive address is still valid offline.",
                            ),
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    box_
}

fn sol_send_view(app_settings: ApplicationSettings, token: Token) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let names: Vec<String> = app_settings
        .sol_wallets
        .iter()
        .map(|wallet| {
            wallet
                .wallet_name
                .clone()
                .unwrap_or_else(|| "Solana".to_string())
        })
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let from_wallet = gtk::DropDown::from_strings(&name_refs);

    let symbol = token.symbol.clone();
    let receive_address = gtk::Entry::builder()
        .placeholder_text("Recipient (base58 address)")
        .hexpand(true)
        .build();
    let amount = gtk::Entry::builder()
        .placeholder_text(&format!("Amount ({symbol})"))
        .hexpand(true)
        .build();

    let review = Button::builder().label("Review send").hexpand(true).build();
    review.add_css_class("standard_button");
    review.add_css_class("suggested-action");

    let error = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-error"])
        .build();

    let confirm_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .css_classes(["receiver-box"])
        .build();
    let summary = gtk::Label::builder()
        .wrap(true)
        .wrap_mode(WrapMode::WordChar)
        .halign(gtk::Align::Start)
        .selectable(true)
        .build();
    let confirm = Button::builder().label("Confirm and broadcast").hexpand(true).build();
    confirm.add_css_class("standard_button");
    confirm.add_css_class("suggested-action");
    let cancel = Button::builder().label("Cancel").hexpand(true).build();
    cancel.add_css_class("standard_button");
    let sol_mainnet = !app_settings.sol_network.eq_ignore_ascii_case("devnet");
    let ack = gtk::CheckButton::with_label("I understand this spends real SOL or tokens.");
    ack.set_visible(sol_mainnet);
    if sol_mainnet {
        confirm.set_sensitive(false);
    }
    let confirm_gate = confirm.clone();
    ack.connect_toggled(move |cb| {
        confirm_gate.set_sensitive(cb.is_active());
    });
    confirm_box.append(&summary);
    confirm_box.append(&ack);
    confirm_box.append(&confirm);
    confirm_box.append(&cancel);

    let status = gtk::Label::builder()
        .wrap(true)
        .visible(false)
        .css_classes(["label-standard"])
        .build();

    box_.append(&from_wallet);
    box_.append(&receive_address);
    box_.append(&amount);
    box_.append(&review);
    box_.append(&error);
    box_.append(&confirm_box);
    box_.append(&status);

    let app_settings = Arc::new(Mutex::new(app_settings));
    let prepared = Arc::new(Mutex::new(None::<sol_chain::PreparedSend>));
    let token = Arc::new(token);

    review.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[strong] token,
        #[weak] receive_address,
        #[weak] amount,
        #[weak] error,
        #[weak] confirm_box,
        #[weak] summary,
        #[weak] from_wallet,
        move |_| {
            error.set_visible(false);
            confirm_box.set_visible(false);
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.sol_wallets.get(index) else {
                error.set_label("Select a Solana account.");
                error.set_visible(true);
                return;
            };
            let Some(from) = wallet.address.clone() else {
                error.set_label("Wallet address is missing.");
                error.set_visible(true);
                return;
            };
            if wallet.private_key.is_none() {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let to = receive_address.text().to_string();
            if sol_chain::validate_address(&to).is_err() {
                error.set_label("Recipient must be a base58 Solana address.");
                error.set_visible(true);
                return;
            }
            let node = settings.sol_node.clone();
            let network_name = settings.sol_network.clone();
            let testnet = network_name.eq_ignore_ascii_case("devnet");
            drop(settings);

            let amount_text = amount.text().to_string();
            let token = (*token).clone();
            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = sol_chain::prepare_send(&from, &to, &amount_text, &token, &node, &network_name);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] error,
                    #[weak] confirm_box,
                    #[weak] summary,
                    #[strong] prepared,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(plan) => {
                                let prefix = if testnet {
                                    "Solana devnet — coins have no mainnet value.\n\n"
                                } else {
                                    "MAINNET: this spends real SOL or tokens.\n\n"
                                };
                                summary.set_label(&format!("{}{}", prefix, plan.summary()));
                                *prepared.lock().unwrap() = Some(plan);
                                confirm_box.set_visible(true);
                            }
                            Err(_) => {
                                error.set_label("Could not build that transaction. Check amount, address, balance, or whether the Solana node is reachable. Receive still works offline.");
                                error.set_visible(true);
                                confirm_box.set_visible(false);
                            }
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    cancel.connect_clicked(clone!(
        #[weak] confirm_box,
        #[strong] prepared,
        move |_| {
            *prepared.lock().unwrap() = None;
            confirm_box.set_visible(false);
        }
    ));

    confirm.connect_clicked(clone!(
        #[strong] app_settings,
        #[strong] prepared,
        #[weak] error,
        #[weak] status,
        #[weak] confirm_box,
        #[weak] from_wallet,
        move |_| {
            let plan = match prepared.lock().unwrap().clone() {
                Some(plan) => plan,
                None => return,
            };
            let settings = app_settings.lock().unwrap();
            let index = from_wallet.selected() as usize;
            let Some(wallet) = settings.sol_wallets.get(index) else {
                return;
            };
            let Some(private_key) = wallet.private_key.clone() else {
                error.set_label("Wallet is locked.");
                error.set_visible(true);
                return;
            };
            let node = settings.sol_node.clone();
            let network_name = settings.sol_network.clone();
            drop(settings);

            error.set_visible(false);
            status.set_label("Broadcasting…");
            status.set_visible(true);
            confirm_box.set_visible(false);

            let (sender, receiver) = crate::configuration::ui_channel::unbounded();
            thread::spawn(move || {
                let result = sol_chain::sign_and_broadcast(&private_key, &plan, &node, &network_name);
                let _ = sender.send_blocking(result);
            });
            crate::configuration::ui_channel::attach(
                receiver,
                clone!(
                    #[weak] status,
                    #[upgrade_or]
                    ControlFlow::Break,
                    move |result| {
                        match result {
                            Ok(txid) => status.set_label(&format!("Sent. Transaction ID: {txid}")),
                            Err(_) => status.set_label(
                                "Broadcast failed. Node may be unreachable; the receive address is still valid offline.",
                            ),
                        }
                        ControlFlow::Break
                    }
                ),
            );
        }
    ));

    box_
}
