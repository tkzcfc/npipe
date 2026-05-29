import request from './request'
import type {
  CleanupDatabaseRequest,
  CleanupDatabaseResponse,
  DatabaseMaintenanceInfoResponse,
  OperationLogRequest,
  OperationLogResponse,
} from '@/types'

export const operationApi = {
  logs(data: OperationLogRequest) {
    return request.post<OperationLogResponse>('/api/operation_logs', data)
  },

  cleanupDatabase(data: CleanupDatabaseRequest) {
    return request.post<CleanupDatabaseResponse>('/api/cleanup_database', data)
  },

  maintenanceInfo(data: CleanupDatabaseRequest) {
    return request.post<DatabaseMaintenanceInfoResponse>('/api/database_maintenance_info', data)
  },
}
