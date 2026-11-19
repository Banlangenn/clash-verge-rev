import { useTheme } from '@mui/material'
import dayjs from 'dayjs'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import type { TrafficSummary } from '@/services/bindings'

import { formatTraffic, formatTrafficShare } from './traffic-value'

interface Props {
  summary: Pick<
    TrafficSummary,
    | 'coreAmount'
    | 'unattributed'
    | 'firstMinute'
    | 'lastMinute'
    | 'lastSampleAt'
    | 'unknownApplication'
    | 'amount'
    | 'estimatedCost'
  >
}

function formatMinute(minute: number) {
  return dayjs.unix(minute).format('YYYY-MM-DD HH:mm')
}

export function TrafficHistoryCoverage({ summary }: Props) {
  const { t } = useTranslation()
  const { palette } = useTheme()
  const {
    coreAmount,
    unattributed,
    firstMinute,
    lastMinute,
    estimatedCost,
    lastSampleAt,
    unknownApplication,
    amount,
  } = summary
  const coreTotal = coreAmount.upload + coreAmount.download
  const missed = unattributed.upload + unattributed.download
  const unknown = unknownApplication.upload + unknownApplication.download
  const routeTotal = amount.upload + amount.download
  const lastSample = lastSampleAt === null ? null : dayjs.unix(lastSampleAt)

  return (
    <div className="mt-2 flex flex-wrap items-center gap-x-2 border-t border-border pt-1 text-xs text-muted-foreground">
      {lastSample && (
        <span>
          {t('connections.history.lastSample', {
            time: lastSample.format(
              lastSample.isSame(dayjs(), 'day') ? 'HH:mm' : 'MM-DD HH:mm',
            ),
          })}
        </span>
      )}
      <Popover>
        <PopoverTrigger asChild>
          <Button variant="ghost" size="sm" className="h-7 text-xs">
            {t('connections.history.dataDetails')}
          </Button>
        </PopoverTrigger>
        <PopoverContent
          align="start"
          aria-label={t('connections.history.dataDetails')}
          data-traffic-theme={palette.mode}
          className="z-[1400] w-80 max-w-[calc(100vw-2rem)] space-y-2 text-sm"
        >
          <dl className="m-0 grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 tabular-nums">
            <dt>{t('connections.history.coreTotal')}</dt>
            <dd className="m-0 text-right font-medium">
              {formatTraffic(coreTotal)}
            </dd>
            <dt>{t('connections.history.unattributed')}</dt>
            <dd className="m-0 text-right font-medium">
              {formatTraffic(missed)}
              {coreTotal > 0 && ' · ' + formatTrafficShare(missed / coreTotal)}
            </dd>
            <dt>{t('connections.history.unknownApplication')}</dt>
            <dd className="m-0 text-right font-medium">
              {formatTraffic(unknown)}
              {routeTotal > 0 &&
                ' · ' + formatTrafficShare(unknown / routeTotal)}
            </dd>
          </dl>
          {firstMinute !== null && lastMinute !== null && (
            <p className="m-0 text-xs text-muted-foreground">
              {t('connections.history.coverage', {
                first: formatMinute(firstMinute),
                last: formatMinute(lastMinute),
              })}
            </p>
          )}
          <p className="m-0">{t('connections.history.dataGapDetail')}</p>
          <p className="m-0">{t('connections.history.coverageScope')}</p>
          <p className="m-0">{t('connections.history.unattributedDetail')}</p>
          <p className="m-0">
            {t('connections.history.unknownApplicationDetail')}
          </p>
          {estimatedCost !== null && (
            <p className="m-0">
              {t('connections.history.estimatedCostDetail')}
            </p>
          )}
        </PopoverContent>
      </Popover>
    </div>
  )
}
