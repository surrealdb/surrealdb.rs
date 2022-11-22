use crate::embedded::Db;
use crate::param::ServerAddrs;
use crate::param::Strict;
use crate::param::ToServerAddrs;
use crate::storage::IndxDb;
use crate::Result;
use url::Url;

impl ToServerAddrs<IndxDb> for &str {
	type Client = Db;

	fn to_server_addrs(self) -> Result<ServerAddrs> {
		Ok(ServerAddrs {
			endpoint: Url::parse(&format!("indxdb://{self}"))?,
			strict: false,
			#[cfg(any(feature = "native-tls", feature = "rustls"))]
			tls_config: None,
		})
	}
}

impl ToServerAddrs<IndxDb> for (&str, Strict) {
	type Client = Db;

	fn to_server_addrs(self) -> Result<ServerAddrs> {
		let mut address = ToServerAddrs::<IndxDb>::to_server_addrs(self.0)?;
		address.strict = true;
		Ok(address)
	}
}
