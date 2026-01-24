<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  createRbacRole,
  deleteRbacRole,
  listRbacRoles,
  setRbacRolePermissions,
  type RbacRoleDto
} from "@/api/rbac/roles";
import { listRbacPermissions, type PermissionDto } from "@/api/rbac/permissions";

defineOptions({
  name: "EmsRbacRoles"
});

const loading = ref(false);
const error = ref("");
const roles = ref<RbacRoleDto[]>([]);
const permissions = ref<PermissionDto[]>([]);

const permissionOptions = computed(() =>
  permissions.value.map(item => ({
    label: `${item.permissionCode} - ${item.description}`,
    value: item.permissionCode
  }))
);

const createDialogOpen = ref(false);
const createForm = ref({
  roleCode: "",
  name: "",
  permissions: [] as string[]
});

const permsDialogOpen = ref(false);
const permsRole = ref<RbacRoleDto | null>(null);
const permsForm = ref({
  permissions: [] as string[]
});

const refresh = async () => {
  error.value = "";
  loading.value = true;
  try {
    const [roleRes, permRes] = await Promise.all([
      listRbacRoles(),
      listRbacPermissions()
    ]);
    if (!permRes.success) {
      error.value = permRes.error?.message ?? "加载权限码失败";
      return;
    }
    permissions.value = permRes.data ?? [];
    if (!roleRes.success) {
      error.value = roleRes.error?.message ?? "加载角色失败";
      return;
    }
    roles.value = roleRes.data ?? [];
  } catch (e) {
    error.value = "请求失败";
  } finally {
    loading.value = false;
  }
};

const openCreate = () => {
  createForm.value = { roleCode: "", name: "", permissions: [] };
  createDialogOpen.value = true;
};

const submitCreate = async () => {
  const roleCode = createForm.value.roleCode.trim();
  const name = createForm.value.name.trim();
  if (!roleCode) {
    ElMessage.error("请输入 roleCode");
    return;
  }
  if (!name) {
    ElMessage.error("请输入名称");
    return;
  }
  loading.value = true;
  try {
    const res = await createRbacRole({
      roleCode,
      name,
      permissions: createForm.value.permissions.length
        ? createForm.value.permissions
        : undefined
    });
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "创建失败");
      return;
    }
    ElMessage.success("创建成功");
    createDialogOpen.value = false;
    await refresh();
  } catch (e) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

const openPerms = (row: RbacRoleDto) => {
  permsRole.value = row;
  permsForm.value = { permissions: [...(row.permissions ?? [])] };
  permsDialogOpen.value = true;
};

const submitPerms = async () => {
  if (!permsRole.value) return;
  loading.value = true;
  try {
    const res = await setRbacRolePermissions(
      permsRole.value.roleCode,
      permsForm.value.permissions
    );
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "更新失败");
      return;
    }
    ElMessage.success("更新成功");
    permsDialogOpen.value = false;
    await refresh();
  } catch (e) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

const removeRole = async (row: RbacRoleDto) => {
  await ElMessageBox.confirm(
    `确定删除角色 ${row.roleCode}？（会级联移除该角色的用户绑定与权限）`,
    "确认",
    { type: "warning" }
  );
  loading.value = true;
  try {
    const res = await deleteRbacRole(row.roleCode);
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "删除失败");
      return;
    }
    ElMessage.success("删除成功");
    await refresh();
  } catch (e) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  refresh();
});
</script>

<template>
  <div>
    <el-card shadow="never" class="mb-4!">
      <template #header>
        <div class="flex items-center justify-between">
          <div>角色管理（RBAC）</div>
          <div>
            <el-button :loading="loading" @click="refresh">刷新</el-button>
            <el-button type="primary" :loading="loading" @click="openCreate">
              新建角色
            </el-button>
          </div>
        </div>
      </template>
      <div v-if="error" class="mb-2 text-red-500">{{ error }}</div>
      <el-table :data="roles" border :loading="loading">
        <el-table-column prop="roleCode" label="roleCode" min-width="200" />
        <el-table-column prop="name" label="name" min-width="200" />
        <el-table-column label="permissions" min-width="240">
          <template #default="{ row }">
            <span>{{ (row.permissions || []).length }}</span>
          </template>
        </el-table-column>
        <el-table-column label="actions" min-width="280">
          <template #default="{ row }">
            <el-button size="small" @click="openPerms(row)">权限</el-button>
            <el-button size="small" type="danger" plain @click="removeRole(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="createDialogOpen" title="新建角色" width="560px">
      <el-form :model="createForm" label-width="120px">
        <el-form-item label="roleCode">
          <el-input v-model="createForm.roleCode" placeholder="例如 operator" />
        </el-form-item>
        <el-form-item label="name">
          <el-input v-model="createForm.name" placeholder="角色名称" />
        </el-form-item>
        <el-form-item label="permissions">
          <el-select
            v-model="createForm.permissions"
            class="w-full"
            multiple
            filterable
            collapse-tags
            collapse-tags-tooltip
          >
            <el-option
              v-for="opt in permissionOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogOpen = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="submitCreate">
          创建
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="permsDialogOpen" title="设置角色权限" width="680px">
      <div class="mb-2 text-sm text-gray-500">
        roleCode: {{ permsRole?.roleCode }}
      </div>
      <el-form :model="permsForm" label-width="120px">
        <el-form-item label="permissions">
          <el-select
            v-model="permsForm.permissions"
            class="w-full"
            multiple
            filterable
            collapse-tags
            collapse-tags-tooltip
          >
            <el-option
              v-for="opt in permissionOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="permsDialogOpen = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="submitPerms">
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

