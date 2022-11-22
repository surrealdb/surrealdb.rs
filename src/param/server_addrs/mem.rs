use crate::embedded::Db;
use crate::param::ServerAddrs;
use crate::param::Strict;
use crate::param::ToServerAddrs;
use crate::storage::Mem;
use crate::Result;
use url::Url;

impl ToServerAddrs<Mem> for () {
	type Client = Db;

	fn to_server_addrs(self) -> Result<ServerAddrs> {
		Ok(ServerAddrs {
			endpoint: Url::parse("mem://")?,
			strict: false,
			#[cfg(any(feature = "native-tls", feature = "rustls"))]
			tls_config: None,
		})
	}
}

impl ToServerAddrs<Mem> for Strict {
	type Client = Db;

	fn to_server_addrs(self) -> Result<ServerAddrs> {
		let mut address = ToServerAddrs::<Mem>::to_server_addrs(())?;
		address.strict = true;
		Ok(address)
	}
}
