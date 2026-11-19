import { useTheme } from '@mui/material'
import dayjs from 'dayjs'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  Bar,
  BarChart,
  CartesianGrid,
  ReferenceArea,
  XAxis,
  YAxis,
} from 'recharts'

import { Button } from '@/components/ui/button'
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart'
import type { TrafficPoint } from '@/services/bindings'

import {
  formatTraffic,
  formatTrafficCost,
  formatTrafficPeriod,
  type TrafficPeriod,
} from './traffic-value'

interface Props extends TrafficPeriod {
  points: TrafficPoint[]
  captureMinutes: number[]
  interruptedMinutes: number[]
  focusedPeriod: TrafficPeriod | null
  onFocus: (period: TrafficPeriod | null) => void
}

const MINUTE = 60
const MAX_BARS = 48

function formatTime(time: number) {
  return dayjs.unix(time).format('HH:mm')
}

export function TrafficHistoryChart({
  from,
  until,
  points,
  captureMinutes,
  interruptedMinutes,
  focusedPeriod,
  onFocus,
}: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const [selectedMinute, setSelectedMinute] = useState<number | null>(null)
  const step = Math.max(
    MINUTE,
    Math.ceil((until - from) / MINUTE / MAX_BARS) * MINUTE,
  )
  const bars = useMemo(() => {
    const captured = new Set(captureMinutes)
    const interrupted = new Set(interruptedMinutes)
    const buckets = Array.from(
      { length: Math.ceil((until - from) / step) },
      (_, index) => {
        const minute = from + index * step
        const end = Math.min(until, minute + step)
        let gap = false
        for (let sample = minute; sample < end; sample += MINUTE) {
          if (!captured.has(sample) || interrupted.has(sample)) {
            gap = true
            break
          }
        }
        return {
          minute,
          upload: 0,
          download: 0,
          estimatedCost: null as number | null,
          gap,
        }
      },
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
  }, [captureMinutes, from, interruptedMinutes, points, step, until])
  const gapPeriods = useMemo(() => {
    const periods: TrafficPeriod[] = []
    for (const bar of bars) {
      if (!bar.gap) continue
      const previous = periods.at(-1)
      if (previous && previous.until + step === bar.minute) {
        previous.until = bar.minute
      } else {
        periods.push({ from: bar.minute, until: bar.minute })
      }
    }
    return periods
  }, [bars, step])
  const highest = bars.reduce<(typeof bars)[number] | undefined>(
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
  const describePeriod = (minute: number): TrafficPeriod => ({
    from: minute,
    until: Math.min(minute + step, until),
  })
  const selectBar = (_bar: unknown, index: number) =>
    setSelectedMinute(bars[index]?.minute ?? null)
  const activePeriod = active ? describePeriod(active.minute) : null
  const focusMatches =
    focusedPeriod?.from === activePeriod?.from &&
    focusedPeriod?.until === activePeriod?.until

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
          {gapPeriods.map((period) => (
            <ReferenceArea
              key={period.from}
              x1={period.from}
              x2={period.until}
              ifOverflow="hidden"
              fill={palette.text.disabled}
              fillOpacity={0.2}
              stroke="none"
              pointerEvents="none"
            />
          ))}
          <XAxis
            dataKey="minute"
            allowDuplicatedCategory={false}
            scale="band"
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
                  const point = payload[0]?.payload as
                    | (typeof bars)[number]
                    | undefined
                  if (!point || !Number.isFinite(point.minute)) return null
                  return (
                    <>
                      {formatTrafficPeriod(describePeriod(point.minute))}
                      {point.gap && (
                        <div className="mt-1 text-muted-foreground">
                          {t('connections.history.dataGap')}
                        </div>
                      )}
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
      {active && activePeriod && (
        <div className="mt-1 flex flex-wrap items-center gap-x-2 px-1 text-xs text-muted-foreground">
          <span>
            {t('connections.history.barDetails', {
              period: formatTrafficPeriod(activePeriod),
              upload: formatTraffic(active.upload),
              download: formatTraffic(active.download),
            })}
            {active.estimatedCost !== null &&
              ` · ${t('connections.history.estimatedCost')} ${formatTrafficCost(active.estimatedCost)}`}
          </span>
          {!focusMatches && active.upload + active.download > 0 && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onFocus(activePeriod)}
            >
              {t('connections.history.viewPeriod')}
            </Button>
          )}
          {focusedPeriod && (
            <Button variant="ghost" size="sm" onClick={() => onFocus(null)}>
              {t('connections.history.backToPeriod')}
            </Button>
          )}
        </div>
      )}
    </>
  )
}
