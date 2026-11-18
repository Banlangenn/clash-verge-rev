import { LinearProgress } from '@mui/material'
import { lazy, Suspense } from 'react'
import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { useVisibility } from '@/hooks/use-visibility'

const TrafficHistoryPanel = lazy(
  () => import('@/components/traffic-history/traffic-history-panel'),
)

export default function TrafficHistoryPage() {
  const { t } = useTranslation()
  const visible = useVisibility()

  return (
    <BasePage title={t('connections.history.title')}>
      <Suspense fallback={<LinearProgress />}>
        <TrafficHistoryPanel visible={visible} />
      </Suspense>
    </BasePage>
  )
}
