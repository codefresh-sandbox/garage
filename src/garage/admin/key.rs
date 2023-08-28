use std::collections::HashMap;

use garage_table::*;

use garage_model::helper::error::Error;
use garage_model::key_table::*;

use crate::cli::*;

use super::*;

impl AdminRpcHandler {
	pub(super) async fn handle_key_cmd(&self, cmd: &KeyOperation) -> Result<AdminRpc, Error> {
		match cmd {
			KeyOperation::List => self.handle_list_keys().await,
			KeyOperation::Info(query) => self.handle_key_info(query).await,
			KeyOperation::New(query) => self.handle_create_key(query).await,
			KeyOperation::Rename(query) => self.handle_rename_key(query).await,
			KeyOperation::Delete(query) => self.handle_delete_key(query).await,
			KeyOperation::Allow(query) => self.handle_allow_key(query).await,
			KeyOperation::Deny(query) => self.handle_deny_key(query).await,
			KeyOperation::Import(query) => self.handle_import_key(query).await,
		}
	}

	async fn handle_list_keys(&self) -> Result<AdminRpc, Error> {
		let key_ids = self
			.garage
			.key_table
			.get_range(
				&EmptyKey,
				None,
				Some(KeyFilter::Deleted(DeletedFilter::NotDeleted)),
				10000,
				EnumerationOrder::Forward,
			)
			.await?
			.iter()
			.map(|k| (k.key_id.to_string(), k.params().unwrap().name.get().clone()))
			.collect::<Vec<_>>();
		Ok(AdminRpc::KeyList(key_ids))
	}

	async fn handle_key_info(&self, query: &KeyOpt) -> Result<AdminRpc, Error> {
		let key = self
			.garage
			.key_helper()
			.get_existing_matching_key(&query.key_pattern)
			.await?;
		self.key_info_result(key).await
	}

	async fn handle_create_key(&self, query: &KeyNewOpt) -> Result<AdminRpc, Error> {
		let key = Key::new(&query.name);
		self.garage.key_table.insert(&key).await?;
		self.key_info_result(key).await
	}

	async fn handle_rename_key(&self, query: &KeyRenameOpt) -> Result<AdminRpc, Error> {
		let mut key = self
			.garage
			.key_helper()
			.get_existing_matching_key(&query.key_pattern)
			.await?;
		key.params_mut()
			.unwrap()
			.name
			.update(query.new_name.clone());
		self.garage.key_table.insert(&key).await?;
		self.key_info_result(key).await
	}

	async fn handle_delete_key(&self, query: &KeyDeleteOpt) -> Result<AdminRpc, Error> {
		let key_helper = self.garage.key_helper();

		let mut key = key_helper
			.get_existing_matching_key(&query.key_pattern)
			.await?;

		if !query.yes {
			return Err(Error::BadRequest(
				"Add --yes flag to really perform this operation".to_string(),
			));
		}

		key_helper.delete_key(&mut key).await?;

		Ok(AdminRpc::Ok(format!(
			"Key {} was deleted successfully.",
			key.key_id
		)))
	}

	async fn handle_allow_key(&self, query: &KeyPermOpt) -> Result<AdminRpc, Error> {
		let mut key = self
			.garage
			.key_helper()
			.get_existing_matching_key(&query.key_pattern)
			.await?;
		if query.create_bucket {
			key.params_mut().unwrap().allow_create_bucket.update(true);
		}
		self.garage.key_table.insert(&key).await?;
		self.key_info_result(key).await
	}

	async fn handle_deny_key(&self, query: &KeyPermOpt) -> Result<AdminRpc, Error> {
		let mut key = self
			.garage
			.key_helper()
			.get_existing_matching_key(&query.key_pattern)
			.await?;
		if query.create_bucket {
			key.params_mut().unwrap().allow_create_bucket.update(false);
		}
		self.garage.key_table.insert(&key).await?;
		self.key_info_result(key).await
	}

	async fn handle_import_key(&self, query: &KeyImportOpt) -> Result<AdminRpc, Error> {
		let prev_key = self.garage.key_table.get(&EmptyKey, &query.key_id).await?;
		if prev_key.is_some() {
			return Err(Error::BadRequest(format!("Key {} already exists in data store. Even if it is deleted, we can't let you create a new key with the same ID. Sorry.", query.key_id)));
		}
		let imported_key = Key::import(&query.key_id, &query.secret_key, &query.name);
		self.garage.key_table.insert(&imported_key).await?;

		self.key_info_result(imported_key).await
	}

	async fn key_info_result(&self, key: Key) -> Result<AdminRpc, Error> {
		let mut relevant_buckets = HashMap::new();

		for (id, _) in key
			.state
			.as_option()
			.unwrap()
			.authorized_buckets
			.items()
			.iter()
		{
			if let Some(b) = self.garage.bucket_table.get(&EmptyKey, id).await? {
				relevant_buckets.insert(*id, b);
			}
		}

		Ok(AdminRpc::KeyInfo(key, relevant_buckets))
	}
}