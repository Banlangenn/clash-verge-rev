import { useTheme } from '@mui/material'
import dayjs from 'dayjs'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from 'recharts'

import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart'
import type { TrafficPoint } from '@/services/bindings'

import { formatTraffic, formatTrafficCost } from './traffic-value'

interface Props {
  from: number
  until: number
  points: TrafficPoint[]
}

const MINUTE = 60
const MAX_BARS = 48

function formatTime(time: number) {
  return dayjs.unix(time).format('HH:mm')
}

export function TrafficHistoryChart({ from, until, points }: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const [selectedMinute, setSelectedMinute] = useState<number | null>(null)
  const step = Math.max(
    MINUTE,
    Math.ceil((until - from) / MINUTE / MAX_BARS) * MINUTE,
  )
  const bars = useMemo(() => {
    const buckets = Array.from(
      { length: Math.ceil((until - from) / step) },
      (_, index): TrafficPoint => ({
        minute: from + index * step,
        upload: 0,
        download: 0,
        estimatedCost: null,
      }),
    )
    for (const point of points) {
      const bucket = buckets[Math.floor((point.minute - from) / step)]
      if (bucket) {
        bucket.upload += point.upload
        bucket.download += point.download
        if (point.estimatedCost !== null) {
          bucket.estimatedCost =
            (bucket.estimatedCost ?? 0) + point.estimatedCost
        }
      }
    }
    return buckets
  }, [from, points, step, until])
  const highest = bars.reduce<TrafficPoint | undefined>(
    (largest, bar) =>
      !largest || bar.upload + bar.download > largest.upload + largest.download
        ? bar
        : largest,
    undefined,
  )
  const active = bars.find((bar) => bar.minute === selectedMinute) ?? highest
  const uploadLabel = t('connections.history.upload')
  const downloadLabel = t('connections.history.download')
  const config = {
    download: { label: downloadLabel, color: palette.primary.main },
    upload: { label: uploadLabel, color: palette.secondary.main },
  }
  const describePeriod = (minute: number) =>
    `${formatTime(minute)}–${formatTime(Math.min(minute + step - MINUTE, until - MINUTE))}`
  const selectBar = (_bar: unknown, index: number) =>
    setSelectedMinute(bars[index]?.minute ?? null)

  return (
    <>
      <ChartContainer
        config={config}
        className="h-[246px] min-w-0 w-full"
        aria-label={t('connections.history.chart', { minutes: step / MINUTE })}
      >
        <BarChart
          accessibilityLayer
          data={bars}
          margin={{ top: 10, right: 14 }}
        >
          <CartesianGrid vertical={false} strokeDasharray="3 4" />
          <XAxis
            dataKey="minute"
            tickFormatter={formatTime}
            minTickGap={23}
            tickLine={false}
            axisLine={false}
          />
          <YAxis
            width={62}
            tickFormatter={(value: number) => formatTraffic(value)}
            tickLine={false}
            axisLine={false}
          />
          <ChartTooltip
            content={
              <ChartTooltipContent
                labelFormatter={(_label, payload) => {
                  const point = payload[0]?.payload as TrafficPoint | undefined
                  if (!point || !Number.isFinite(point.minute)) return null
                  return (
                    <>
                      {describePeriod(point.minute)}
                      {point.estimatedCost !== null && (
                        <div className="mt-1.5 flex justify-between gap-2">
                          <span className="font-normal text-muted-foreground">
                            {t('connections.history.estimatedCost')}
                          </span>
                          <span className="tabular-nums">
                            {formatTrafficCost(point.estimatedCost)}
                          </span>
                        </div>
                      )}
                    </>
                  )
                }}
                formatter={(value, name) => (
                  <div className="flex w-full items-center justify-between gap-2">
                    <span className="text-muted-foreground">
                      {name === 'upload' ? uploadLabel : downloadLabel}
                    </span>
                    <span className="font-medium tabular-nums">
                      {formatTraffic(Number(value))}
                    </span>
                  </div>
                )}
              />
            }
          />
          <ChartLegend content={<ChartLegendContent />} />
          <Bar
            dataKey="download"
            stackId="traffic"
            fill="var(--color-download)"
            isAnimationActive={false}
            onClick={selectBar}
          />
          <Bar
            dataKey="upload"
            stackId="traffic"
            fill="var(--color-upload)"
            radius={[3, 3, 0, 0]}
            isAnimationActive={false}
            onClick={selectBar}
          />
        </BarChart>
      </ChartContainer>
      {active && (
        <p className="mt-1 px-1 text-xs text-muted-foreground">
          {t('connections.history.bar', {
            from: formatTime(active.minute),
            until: formatTime(
              Math.min(active.minute + step - MINUTE, until - MINUTE),
            ),
            upload: formatTraffic(active.upload),
            download: formatTraffic(active.download),
          })}
          {active.estimatedCost !== null &&
            ` · ${t('connections.history.estimatedCost')} ${formatTrafficCost(active.estimatedCost)}`}
        </p>
      )}
    </>
  )
}
