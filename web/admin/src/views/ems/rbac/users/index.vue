<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  createRbacUser,
  listRbacUsers,
  setRbacUserRoles,
  updateRbacUser,
  type RbacUserDto
} from "@/api/rbac/users";
import { listRbacRoles, type RbacRoleDto } from "@/api/rbac/roles";

defineOptions({
  name: "EmsRbacUsers"
});

const loading = ref(false);
const error = ref("");
const users = ref<RbacUserDto[]>([]);
const roles = ref<RbacRoleDto[]>([]);

const roleOptions = computed(() =>
  roles.value.map(item => ({ label: `${item.roleCode} (${item.name})`, value: item.roleCode }))
);

const createDialogOpen = ref(false);
const createForm = ref({
  username: "",
  password: "",
  status: "active",
  roles: [] as string[]
});

const editDialogOpen = ref(false);
const editUser = ref<RbacUserDto | null>(null);
const editForm = ref({
  status: "",
  password: ""
});

const rolesDialogOpen = ref(false);
const rolesUser = ref<RbacUserDto | null>(null);
const rolesForm = ref({
  roles: [] as string[]
});

const refresh = async () => {
  error.value = "";
  loading.value = true;
  try {
    const [userRes, roleRes] = await Promise.all([listRbacUsers(), listRbacRoles()]);
    if (!roleRes.success) {
      error.value = roleRes.error?.message ?? "加载角色失败";
      return;
    }
    roles.value = roleRes.data ?? [];
    if (!userRes.success) {
      error.value = userRes.error?.message ?? "加载用户失败";
      return;
    }
    users.value = userRes.data ?? [];
  } catch (e) {
    error.value = "请求失败";
  } finally {
    loading.value = false;
  }
};

const openCreate = () => {
  createForm.value = { username: "", password: "", status: "active", roles: [] };
  createDialogOpen.value = true;
};

const submitCreate = async () => {
  const username = createForm.value.username.trim();
  const password = createForm.value.password.trim();
  if (!username) {
    ElMessage.error("请输入用户名");
    return;
  }
  if (!password) {
    ElMessage.error("请输入密码");
    return;
  }
  loading.value = true;
  try {
    const res = await createRbacUser({
      username,
      password,
      status: createForm.value.status.trim() || undefined,
      roles: createForm.value.roles.length ? createForm.value.roles : undefined
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

const openEdit = (row: RbacUserDto) => {
  editUser.value = row;
  editForm.value = { status: row.status, password: "" };
  editDialogOpen.value = true;
};

const submitEdit = async () => {
  if (!editUser.value) return;
  const status = editForm.value.status.trim();
  const password = editForm.value.password.trim();
  if (!status) {
    ElMessage.error("status 不能为空");
    return;
  }
  const payload: { status?: string; password?: string } = { status };
  if (password) payload.password = password;
  loading.value = true;
  try {
    const res = await updateRbacUser(editUser.value.userId, payload);
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "更新失败");
      return;
    }
    ElMessage.success("更新成功");
    editDialogOpen.value = false;
    await refresh();
  } catch (e) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

const openRoles = (row: RbacUserDto) => {
  rolesUser.value = row;
  rolesForm.value = { roles: [...(row.roles ?? [])] };
  rolesDialogOpen.value = true;
};

const submitRoles = async () => {
  if (!rolesUser.value) return;
  loading.value = true;
  try {
    const res = await setRbacUserRoles(rolesUser.value.userId, rolesForm.value.roles);
    if (!res.success) {
      ElMessage.error(res.error?.message ?? "更新失败");
      return;
    }
    ElMessage.success("更新成功");
    rolesDialogOpen.value = false;
    await refresh();
  } catch (e) {
    ElMessage.error("请求失败");
  } finally {
    loading.value = false;
  }
};

const confirmDeleteHint = async () => {
  await ElMessageBox.alert(
    "当前 MVP 未开放删除用户接口（避免误删）。如需支持可再补。",
    "提示",
    { type: "info" }
  );
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
          <div>用户管理（RBAC）</div>
          <div>
            <el-button :loading="loading" @click="refresh">刷新</el-button>
            <el-button type="primary" :loading="loading" @click="openCreate">
              新建用户
            </el-button>
          </div>
        </div>
      </template>
      <div v-if="error" class="mb-2 text-red-500">{{ error }}</div>
      <el-table :data="users" border :loading="loading">
        <el-table-column prop="userId" label="userId" min-width="240" />
        <el-table-column prop="username" label="username" min-width="160" />
        <el-table-column prop="status" label="status" min-width="120" />
        <el-table-column label="roles" min-width="260">
          <template #default="{ row }">
            <el-tag
              v-for="item in row.roles || []"
              :key="item"
              class="mr-1"
              effect="light"
            >
              {{ item }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="actions" min-width="260">
          <template #default="{ row }">
            <el-button size="small" @click="openEdit(row)">编辑</el-button>
            <el-button size="small" @click="openRoles(row)">角色</el-button>
            <el-button size="small" type="danger" plain @click="confirmDeleteHint">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="createDialogOpen" title="新建用户" width="520px">
      <el-form :model="createForm" label-width="120px">
        <el-form-item label="username">
          <el-input v-model="createForm.username" placeholder="用户名" />
        </el-form-item>
        <el-form-item label="password">
          <el-input
            v-model="createForm.password"
            type="password"
            show-password
            placeholder="初始密码"
          />
        </el-form-item>
        <el-form-item label="status">
          <el-select v-model="createForm.status" class="w-full">
            <el-option label="active" value="active" />
            <el-option label="disabled" value="disabled" />
          </el-select>
        </el-form-item>
        <el-form-item label="roles">
          <el-select
            v-model="createForm.roles"
            class="w-full"
            multiple
            filterable
            collapse-tags
            collapse-tags-tooltip
          >
            <el-option
              v-for="opt in roleOptions"
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

    <el-dialog v-model="editDialogOpen" title="编辑用户" width="520px">
      <el-form :model="editForm" label-width="120px">
        <el-form-item label="status">
          <el-select v-model="editForm.status" class="w-full">
            <el-option label="active" value="active" />
            <el-option label="disabled" value="disabled" />
          </el-select>
        </el-form-item>
        <el-form-item label="password">
          <el-input
            v-model="editForm.password"
            type="password"
            show-password
            placeholder="留空则不修改"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogOpen = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="submitEdit">
          保存
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="rolesDialogOpen" title="设置用户角色" width="520px">
      <el-form :model="rolesForm" label-width="120px">
        <el-form-item label="roles">
          <el-select
            v-model="rolesForm.roles"
            class="w-full"
            multiple
            filterable
            collapse-tags
            collapse-tags-tooltip
          >
            <el-option
              v-for="opt in roleOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rolesDialogOpen = false">取消</el-button>
        <el-button type="primary" :loading="loading" @click="submitRoles">
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

