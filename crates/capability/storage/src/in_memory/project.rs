//! 项目内存存储实现
//!
//! 仅用于本地 M0 演示和测试。
//!
//! 功能：
//! - 内置默认项目（project-1，Default Project）
//! - 项目 CRUD 操作
//! - 租户隔离验证

use crate::error::StorageError;
use crate::models::{ProjectRecord, ProjectUpdate};
use crate::traits::ProjectStore;
use crate::validation::ensure_tenant;
use domain::TenantContext;
use std::collections::HashMap;
use std::sync::RwLock;

/// 项目内存存储
///
/// 使用 RwLock + HashMap 提供线程安全的内存存储。
pub struct InMemoryProjectStore {
    projects: RwLock<HashMap<String, ProjectRecord>>,
}

impl InMemoryProjectStore {
    /// 内置默认项目
    ///
    /// 创建包含默认项目的存储。
    pub fn with_default_project() -> Self {
        let mut projects = HashMap::new();
        projects.insert(
            "project-1".to_string(),
            ProjectRecord {
                project_id: "project-1".to_string(),
                tenant_id: "tenant-1".to_string(),
                name: "Default Project".to_string(),
                timezone: "UTC".to_string(),
            },
        );
        Self {
            projects: RwLock::new(projects),
        }
    }
}

#[async_trait::async_trait]
impl ProjectStore for InMemoryProjectStore {
    /// 列出当前租户的所有项目
    async fn list_projects(&self, ctx: &TenantContext) -> Result<Vec<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let projects = self
            .projects
            .read()
            .map(|map| {
                map.values()
                    .filter(|project| project.tenant_id == ctx.tenant_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(projects)
    }

    /// 查找指定项目
    async fn find_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<Option<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let project = self
            .projects
            .read()
            .ok()
            .and_then(|map| map.get(project_id).cloned())
            .filter(|project| project.tenant_id == ctx.tenant_id);
        Ok(project)
    }

    /// 创建新项目
    async fn create_project(
        &self,
        ctx: &TenantContext,
        record: ProjectRecord,
    ) -> Result<ProjectRecord, StorageError> {
        ensure_tenant(ctx)?;
        if record.tenant_id != ctx.tenant_id {
            return Err(StorageError::new("tenant mismatch"));
        }
        let mut map = self
            .projects
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        if map.contains_key(&record.project_id) {
            return Err(StorageError::new("project exists"));
        }
        map.insert(record.project_id.clone(), record.clone());
        Ok(record)
    }

    /// 更新项目
    async fn update_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
        update: ProjectUpdate,
    ) -> Result<Option<ProjectRecord>, StorageError> {
        ensure_tenant(ctx)?;
        let mut map = self
            .projects
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        let project = match map.get_mut(project_id) {
            Some(project) => project,
            None => return Ok(None),
        };
        if project.tenant_id != ctx.tenant_id {
            return Ok(None);
        }
        if let Some(name) = update.name {
            project.name = name;
        }
        if let Some(timezone) = update.timezone {
            project.timezone = timezone;
        }
        Ok(Some(project.clone()))
    }

    /// 删除项目
    async fn delete_project(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_tenant(ctx)?;
        let mut map = self
            .projects
            .write()
            .map_err(|_| StorageError::new("lock failed"))?;
        match map.get(project_id) {
            Some(project) if project.tenant_id == ctx.tenant_id => {
                map.remove(project_id);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// 验证项目归属当前租户
    async fn project_belongs_to_tenant(
        &self,
        ctx: &TenantContext,
        project_id: &str,
    ) -> Result<bool, StorageError> {
        ensure_tenant(ctx)?;
        let matched = match self.projects.read() {
            Ok(map) => map
                .get(project_id)
                .map(|project| project.tenant_id == ctx.tenant_id)
                .unwrap_or(false),
            Err(_) => false,
        };
        Ok(matched)
    }
}
