import DownloadOutlined from '@mui/icons-material/DownloadOutlined'
import SwapVertOutlined from '@mui/icons-material/SwapVertOutlined'
import UploadOutlined from '@mui/icons-material/UploadOutlined'
import {
  Alert,
  Box,
  LinearProgress,
  Paper,
  Stack,
  Typography,
  alpha,
  useTheme,
} from '@mui/material'
import dayjs from 'dayjs'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { TrafficRoute } from '@/services/bindings'
import { getTrafficHistory } from '@/services/cmds'
import { useQuery } from '@/services/query-client'

import {
  TrafficHistoryBreakdown,
  type TrafficGrouping,
} from './traffic-history-breakdown'
import { TrafficHistoryChart } from './traffic-history-chart'
import { TrafficHistoryCoverage } from './traffic-history-coverage'
import {
  DAY_FORMAT,
  TrafficHistoryFilters,
  type TrafficRange,
} from './traffic-history-filters'
import {
  formatTraffic,
  formatTrafficCost,
  type TrafficPeriod,
} from './traffic-value'

import './traffic-history.css'

const MINUTE = 60

const metricIcons = [
  { icon: SwapVertOutlined, color: 'info' },
  { icon: UploadOutlined, color: 'secondary' },
  { icon: DownloadOutlined, color: 'primary' },
] as const

interface Props {
  visible: boolean
}

export default function TrafficHistoryPanel({ visible }: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const today = dayjs().format(DAY_FORMAT)
  const [range, setRange] = useState<TrafficRange>({
    date: today,
    startTime: '00:00',
    endTime: '23:59',
  })
  const { date, startTime, endTime } = range
  const [recentMinutes, setRecentMinutes] = useState<number | null>(null)
  const [focusedPeriod, setFocusedPeriod] = useState<TrafficPeriod | null>(null)
  const [route, setRoute] = useState<TrafficRoute>('proxy')
  const [grouping, setGrouping] = useState<TrafficGrouping>({
    dimension: 'application',
    application: null,
  })
  const { dimension, application } = grouping
  const from = dayjs(`${date}T${startTime}`).unix()
  const until = dayjs(`${date}T${endTime}`).unix() + MINUTE
  const validRange =
    !!date && !!startTime && !!endTime && Number.isFinite(from) && from < until
  const live = recentMinutes !== null || date === today
  const {
    data: result,
    error,
    isPending,
  } = useQuery({
    queryKey: [
      'traffic-history',
      recentMinutes ?? from,
      recentMinutes === null ? until : 'recent',
      route,
      dimension,
      application,
    ],
    queryFn: async () => {
      const recentUntil = dayjs().startOf('minute').unix() + MINUTE
      const start =
        recentMinutes === null ? from : recentUntil - recentMinutes * MINUTE
      const end = recentMinutes === null ? until : recentUntil
      const summary = await getTrafficHistory(
        start,
        end,
        route,
        dimension,
        application,
      )
      return { summary, from: start, until: end, recentUntil }
    },
    enabled: visible && (validRange || recentMinutes !== null),
    refetchInterval: live ? 10_000 : false,
    retry: false,
  })
  const data = result?.summary
  const focusedQuery = useQuery({
    queryKey: [
      'traffic-history-ranking',
      focusedPeriod?.from,
      focusedPeriod?.until,
      route,
      dimension,
      application,
    ],
    queryFn: () => {
      if (!focusedPeriod) throw new Error('Missing traffic ranking period')
      return getTrafficHistory(
        focusedPeriod.from,
        focusedPeriod.until,
        route,
        dimension,
        application,
      )
    },
    enabled: visible && focusedPeriod !== null,
    refetchInterval: live ? 10_000 : false,
    retry: false,
  })
  const ranking = focusedPeriod ? focusedQuery.data : data

  const clearApplication = () =>
    setGrouping({ dimension: 'application', application: null })

  const total = (ranking?.amount.upload ?? 0) + (ranking?.amount.download ?? 0)

  return (
    <Stack
      data-traffic-theme={palette.mode}
      spacing={1.5}
      sx={{ width: '100%', maxWidth: 1280, mx: 'auto', pb: 1 }}
    >
      <TrafficHistoryFilters
        range={range}
        onRangeChange={(next) => {
          setRange(next)
          setRecentMinutes(null)
          setFocusedPeriod(null)
        }}
        recentMinutes={recentMinutes}
        onRecentChange={(minutes) => {
          setRecentMinutes(minutes)
          setFocusedPeriod(null)
        }}
        route={route}
        onRouteChange={(next) => {
          setRoute(next)
          setFocusedPeriod(null)
        }}
        application={application}
        onClearApplication={clearApplication}
      />
      {!validRange && (
        <Alert severity="info">{t('connections.history.invalidRange')}</Alert>
      )}
      {isPending && (
        <LinearProgress aria-label={t('connections.history.title')} />
      )}
      {error && (
        <Alert severity="error">{t('connections.history.readError')}</Alert>
      )}
      {data && result && (
        <>
          <Box
            sx={{
              display: 'grid',
              gap: 1.25,
              gridTemplateColumns:
                'repeat(auto-fit, minmax(min(100%, 145px), 1fr))',
            }}
          >
            {[
              {
                label: t('connections.history.routeTotal', {
                  route: t(
                    route === 'proxy'
                      ? 'connections.history.proxy'
                      : 'connections.history.direct',
                  ),
                }),
                value: formatTraffic(total),
              },
              {
                label: t('connections.history.upload'),
                value: formatTraffic(data.amount.upload),
              },
              {
                label: t('connections.history.download'),
                value: formatTraffic(data.amount.download),
              },
              ...(data.estimatedCost !== null
                ? [
                    {
                      label: t('connections.history.estimatedCost'),
                      value: formatTrafficCost(data.estimatedCost),
                    },
                  ]
                : []),
            ].map(({ label, value }, index) => {
              const metric = metricIcons[index]
              const Icon = metric?.icon
              return (
                <Paper
                  key={label}
                  variant="outlined"
                  sx={{ px: 1.5, py: 1.25, borderRadius: 2 }}
                >
                  <Stack
                    direction="row"
                    spacing={1.25}
                    sx={{ alignItems: 'center' }}
                  >
                    {metric && Icon && (
                      <Box
                        sx={(theme) => ({
                          display: 'grid',
                          placeItems: 'center',
                          flex: '0 0 36px',
                          height: 36,
                          borderRadius: 1.25,
                          color: theme.palette[metric.color].main,
                          bgcolor: alpha(theme.palette[metric.color].main, 0.1),
                        })}
                      >
                        <Icon fontSize="small" />
                      </Box>
                    )}
                    <Box sx={{ minWidth: 0 }}>
                      <Typography variant="body2" color="text.secondary">
                        {label}
                      </Typography>
                      <Typography
                        variant="h6"
                        sx={{
                          fontWeight: 700,
                          fontVariantNumeric: 'tabular-nums',
                          overflowWrap: 'anywhere',
                        }}
                      >
                        {value}
                      </Typography>
                    </Box>
                  </Stack>
                </Paper>
              )
            })}
          </Box>
          <Paper variant="outlined" sx={{ p: 1.5, borderRadius: 2 }}>
            <Typography
              component="h2"
              variant="subtitle1"
              sx={{ fontWeight: 700, mb: 0.75 }}
            >
              {t('connections.history.trend')}
            </Typography>
            {(data.points.length > 0 || data.sampledMinutes > 0) &&
            data.lastMinute !== null ? (
              <TrafficHistoryChart
                from={
                  recentMinutes === null
                    ? Math.max(result.from, data.firstMinute ?? result.from)
                    : result.from
                }
                until={Math.min(result.until, result.recentUntil)}
                points={data.points}
                captureMinutes={data.captureMinutes}
                interruptedMinutes={data.interruptedMinutes}
                focusedPeriod={focusedPeriod}
                onFocus={setFocusedPeriod}
              />
            ) : (
              <Typography variant="body2" color="text.secondary" sx={{ py: 2 }}>
                {t(
                  data.sampledMinutes === 0
                    ? 'connections.history.noSamples'
                    : 'connections.history.noTraffic',
                )}
              </Typography>
            )}
            <TrafficHistoryCoverage summary={data} />
          </Paper>
          {focusedPeriod && focusedQuery.isPending && (
            <LinearProgress aria-label={t('connections.history.breakdown')} />
          )}
          {focusedPeriod && focusedQuery.error && (
            <Alert severity="error">{t('connections.history.readError')}</Alert>
          )}
          {ranking && (
            <TrafficHistoryBreakdown
              total={total}
              rows={ranking.breakdown}
              costRows={ranking.costBreakdown}
              grouping={grouping}
              onGroupingChange={setGrouping}
              onClearApplication={clearApplication}
              focusedPeriod={focusedPeriod}
            />
          )}
        </>
      )}
    </Stack>
  )
}
