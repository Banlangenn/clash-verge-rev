import { useTheme } from '@mui/material'
import { useTranslation } from 'react-i18next'
import { Pie, PieChart } from 'recharts'

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart'
import type { TrafficBreakdown } from '@/services/bindings'

import { formatTraffic, formatTrafficShare } from './traffic-value'

const lightColors = ['#2563eb', '#00815e', '#bd630c', '#ba3b74', '#7044b6']
const darkColors = ['#82acff', '#54d6a7', '#ffbc66', '#f58fba', '#c2a6ff']

interface Props {
  rows: TrafficBreakdown[]
  total: number
  unknownLabel: string
}

export function TrafficHistoryShare({ rows, total, unknownLabel }: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const colors = palette.mode === 'dark' ? darkColors : lightColors
  const top = rows.slice(0, colors.length).map((row, index) => ({
    id: index,
    label: row.name || unknownLabel,
    value: row.upload + row.download,
    fill: colors[index],
  }))
  const remaining = total - top.reduce((sum, slice) => sum + slice.value, 0)
  const slices =
    remaining > 0
      ? [
          ...top,
          {
            id: colors.length,
            label: t('connections.history.other'),
            value: remaining,
            fill: palette.mode === 'dark' ? '#a1a1aa' : '#6b7280',
          },
        ]
      : top
  const formatShare = (value: number) => formatTrafficShare(value / total)

  return (
    <section className="min-w-0 border-t border-border px-3 pt-3 pb-3.5 md:border-t-0 md:border-l">
      <h3 className="m-0 text-sm font-semibold">
        {t('connections.history.share')}
      </h3>
      <ChartContainer
        className="h-[173px] w-full"
        config={{ value: { label: t('connections.history.total') } }}
        aria-label={t('connections.history.share')}
      >
        <PieChart accessibilityLayer>
          <ChartTooltip
            content={
              <ChartTooltipContent
                hideLabel
                formatter={(value, name) => (
                  <div className="grid gap-1">
                    <span className="font-medium">{name}</span>
                    <span className="tabular-nums">
                      {formatTraffic(Number(value))} ·{' '}
                      {formatShare(Number(value))}
                    </span>
                  </div>
                )}
              />
            }
          />
          <Pie
            data={slices}
            dataKey="value"
            nameKey="label"
            innerRadius="62%"
            outerRadius="86%"
            paddingAngle={2}
            stroke="none"
            isAnimationActive={false}
          />
        </PieChart>
      </ChartContainer>
      <ul className="m-0 grid list-none gap-1.5 pl-0">
        {slices.map(({ id, label, fill, value }) => (
          <li key={id} className="flex min-w-0 items-center gap-2 text-sm">
            <span
              className="size-2 shrink-0 rounded-full"
              style={{ backgroundColor: fill }}
              aria-hidden
            />
            <span className="flex-1 truncate" title={label}>
              {label}
            </span>
            <span className="text-muted-foreground tabular-nums">
              {formatShare(value)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  )
}
