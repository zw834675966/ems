import { storageLocal } from "@pureadmin/utils";

const EMS_PROJECT_ID_KEY = "ems-project-id";

export function loadEmsProjectId(): string {
  return (storageLocal().getItem(EMS_PROJECT_ID_KEY) as string) || "";
}

export function saveEmsProjectId(projectId: string) {
  storageLocal().setItem(EMS_PROJECT_ID_KEY, projectId);
}

export function clearEmsProjectId() {
  storageLocal().removeItem(EMS_PROJECT_ID_KEY);
}

