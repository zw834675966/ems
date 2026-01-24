import { ref, computed, onMounted, watch, type WatchSource } from "vue";
import { type PaginationProps } from "@pureadmin/table";
import { ElMessage, ElMessageBox } from "element-plus";

type ApiResult<T> = {
    success: boolean;
    data?: T;
    error?: {
        message: string;
        [key: string]: any;
    };
};

export function useCrud<T, Q extends Record<string, any> = any>(
    listApi: (params?: Q) => Promise<ApiResult<T[]>>,
    deleteApi?: (id: string) => Promise<ApiResult<void>>,
    options?: {
        immediate?: boolean; // Whether to fetch data on mount
        idKey?: string; // The key property for deletion (default: 'id')
        watchSource?: WatchSource | WatchSource[]; // Source to watch for auto-refresh
        onFetchSuccess?: (data: T[]) => void;
    }
) {
    const loading = ref(false);
    const dataList = ref<T[]>([]) as any;
    const error = ref("");
    const searchValue = ref("");

    // Pagination state
    const pagination = ref<PaginationProps>({
        total: 0,
        pageSize: 10,
        currentPage: 1,
        background: true
    });

    // Client-side filtering and pagination
    const filteredData = computed(() => {
        let result = [...dataList.value];

        // Filter by search value (simple string match on all string fields)
        if (searchValue.value) {
            const lowerSearch = searchValue.value.toLowerCase();
            result = result.filter(item => {
                return Object.values(item as any).some(val =>
                    String(val).toLowerCase().includes(lowerSearch)
                );
            });
        }

        // Update total count for pagination
        pagination.value.total = result.length;

        // Slice for pagination
        const start = (pagination.value.currentPage - 1) * pagination.value.pageSize;
        const end = start + pagination.value.pageSize;
        return result.slice(start, end);
    });

    const fetchList = async () => {
        loading.value = true;
        error.value = "";
        try {
            const res = await listApi();
            if (res.success) {
                dataList.value = res.data ?? [];
                options?.onFetchSuccess?.(dataList.value);
            } else {
                error.value = res.error?.message ?? "加载失败";
                ElMessage.error(error.value);
            }
        } catch (err: any) {
            error.value = err.message || "请求失败";
        } finally {
            loading.value = false;
        }
    };

    const handleDelete = (row: T) => {
        if (!deleteApi) return;

        const name = (row as any).name || (row as any).title || "该项";
        const idKey = options?.idKey || "id";
        const id = (row as any)[idKey];

        ElMessageBox.confirm(`确认删除 "${name}" 吗？此操作不可撤销。`, "警告", {
            confirmButtonText: "确定",
            cancelButtonText: "取消",
            type: "warning"
        }).then(async () => {
            loading.value = true;
            try {
                const res = await deleteApi(id);
                if (res.success) {
                    ElMessage.success("删除成功");
                    if (filteredData.value.length === 1 && pagination.value.currentPage > 1) {
                        pagination.value.currentPage--;
                    }
                    await fetchList();
                } else {
                    ElMessage.error(res.error?.message ?? "删除失败");
                }
            } catch (err) {
                ElMessage.error("请求失败");
            } finally {
                loading.value = false;
            }
        });
    };

    const onPageSizeChange = (val: number) => {
        pagination.value.pageSize = val;
        pagination.value.currentPage = 1;
    };

    const onCurrentPageChange = (val: number) => {
        pagination.value.currentPage = val;
    };

    onMounted(() => {
        if (options?.immediate !== false) {
            fetchList();
        }
    });

    if (options?.watchSource) {
        watch(options.watchSource, (val) => {
            if (val) {
                fetchList();
            } else {
                dataList.value = [];
            }
        });
    }

    return {
        loading,
        error,
        dataList,
        filteredData,
        searchValue,
        pagination,
        fetchList,
        handleDelete,
        onPageSizeChange,
        onCurrentPageChange
    };
}

