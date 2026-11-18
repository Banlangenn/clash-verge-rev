import { useTheme } from '@mui/material'
import { useTranslation } from 'react-i18next'
import { Pie, PieChart } from 'recharts'

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from '@/components/ui/chart'
import type { TrafficBreakdown } from '@/services/bindings'

import { formatTraffic } from './traffic-value'

interface Props {
  rows: TrafficBreakdown[]
  total: number
}

export function TrafficHistoryShare({ rows, total }: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const colors = [
    palette.primary.main,
    palette.secondary.main,
    palette.info.main,
    palette.success.main,
    palette.warning.main,
  ]
  const top = rows.slice(0, colors.length).map((row, index) => ({
    id: index,
    label: row.name || t('connections.history.unknown'),
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
            fill: palette.text.disabled,
          },
        ]
      : top
  const percent = new Intl.NumberFormat(undefined, {
    style: 'percent',
    maximumFractionDigits: 1,
  })
  const formatShare = (value: number) => percent.format(value / total)

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
