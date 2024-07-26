use std::{
    env,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

use anyhow::anyhow;
use axum::http::HeaderValue;
use cln_plugin::{
    options::{ConfigOption, DefaultStringConfigOption, IntegerConfigOption, StringConfigOption},
    ConfiguredPlugin,
};
use tower_http::cors::{Any, CorsLayer};

use crate::PluginState;

pub const OPT_CLNREST_PORT: IntegerConfigOption =
    ConfigOption::new_i64_no_default("clnrest-port", "REST server port to listen");
pub const OPT_CLNREST_CERTS: StringConfigOption =
    ConfigOption::new_str_no_default("clnrest-certs", "Path for certificates (for https)");
pub const OPT_CLNREST_PROTOCOL: DefaultStringConfigOption =
    ConfigOption::new_str_with_default("clnrest-protocol", "https", "REST server protocol");
pub const OPT_CLNREST_HOST: DefaultStringConfigOption =
    ConfigOption::new_str_with_default("clnrest-host", "127.0.0.1", "REST server host");
pub const OPT_CLNREST_CORS: DefaultStringConfigOption = ConfigOption::new_str_with_default(
    "clnrest-cors-origins",
    "*",
    "Cross origin resource sharing origins",
);
pub const OPT_CLNREST_CSP: DefaultStringConfigOption = ConfigOption::new_str_with_default(
    "clnrest-csp",
    "default-src 'self'; font-src 'self'; img-src 'self' data:; frame-src 'self'; \
    style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline';",
    "Content security policy (CSP) for the server",
);
pub const OPT_CLNREST_SWAGGER: DefaultStringConfigOption =
    ConfigOption::new_str_with_default("clnrest-swagger-root", "/", "Root path for Swagger UI");

pub enum ClnrestProtocol {
    Https,
    Http,
}
pub struct ClnrestOptions {
    pub certs: PathBuf,
    pub protocol: ClnrestProtocol,
    pub address_str: String,
    pub address: SocketAddr,
    pub cors: CorsLayer,
    pub csp: String,
}

pub async fn parse_options(
    plugin: &ConfiguredPlugin<PluginState, tokio::io::Stdin, tokio::io::Stdout>,
) -> Result<ClnrestOptions, anyhow::Error> {
    let port = if let Some(p) = plugin.option(&OPT_CLNREST_PORT)? {
        if !(1024..=65535).contains(&p) {
            // plugin
            //     .disable(&format!(
            //         "`clnrest-port` {}, should be a valid available port between 1024 and 65535.",
            //         p
            //     ))
            //     .await?;
            return Err(anyhow!(
                "`clnrest-port` {}, should be a valid available port between 1024 and 65535.",
                p
            ));
        }
        p as u16
    } else {
        log::info!("`clnrest-port` option is not configured");
        // return plugin
        //     .disable("`clnrest-port` option is not configured")
        //     .await;
        return Err(anyhow!("`clnrest-port` option is not configured"));
    };

    let protocol = match plugin.option(&OPT_CLNREST_PROTOCOL)? {
        p if p.eq_ignore_ascii_case("https") => ClnrestProtocol::Https,
        p if p.eq_ignore_ascii_case("http") => ClnrestProtocol::Http,
        _ => {
            // return plugin
            //     .disable("`clnrest-protocol` can either be http or https.")
            //     .await;
            return Err(anyhow!("`clnrest-protocol` can either be http or https."));
        }
    };

    let address_str = format!("{}:{}", plugin.option(&OPT_CLNREST_HOST)?, port);
    let address: SocketAddr = match plugin.option(&OPT_CLNREST_HOST)? {
        i if i.eq("localhost") => address_str
            .to_socket_addrs()?
            .next()
            .ok_or(anyhow!("No address found for localhost"))?,
        _ => {
            if let Ok(a) = address_str.parse() {
                a
            } else {
                // return plugin.disable("`clnrest-host` should be a valid IP.").await;
                return Err(anyhow!("`clnrest-host` should be a valid IP."));
            }
        }
    };
    let cors = create_cors_layer(&plugin.option(&OPT_CLNREST_CORS)?)?;

    let certs = if let Some(cert_opt) = plugin.option(&OPT_CLNREST_CERTS)? {
        PathBuf::from(cert_opt)
    } else {
        env::current_dir()?
    };

    let csp = plugin.option(&OPT_CLNREST_CSP)?;

    Ok(ClnrestOptions {
        certs,
        protocol,
        address_str,
        address,
        cors,
        csp,
    })
}

fn create_cors_layer(allowed_origin: &str) -> Result<CorsLayer, anyhow::Error> {
    let cors = if allowed_origin == "*" {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins = allowed_origin
            .chars()
            .filter(|x| !x.is_whitespace())
            .collect::<String>()
            .split(',')
            .collect::<Vec<&str>>()
            .iter()
            .map(|y| y.parse::<HeaderValue>().unwrap())
            .collect::<Vec<HeaderValue>>();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    };
    log::debug!("cors_layer: in:{} out:{:?}", allowed_origin, cors);
    Ok(cors)
}