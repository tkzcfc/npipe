import request from './request'
import type { DashboardOverviewResponse } from '@/types'

export const dashboardApi = {
  overview() {
    return request.post<DashboardOverviewResponse>('/api/dashboard_overview')
  },
}
