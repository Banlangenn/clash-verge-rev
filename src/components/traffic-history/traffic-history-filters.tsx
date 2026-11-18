import { useTheme } from '@mui/material'
import { zhCN } from 'date-fns/locale/zh-CN'
import { zhTW } from 'date-fns/locale/zh-TW'
import dayjs from 'dayjs'
import { CalendarDays, Clock3, X } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'
import { Calendar } from '@/components/ui/calendar'
import { Input } from '@/components/ui/input'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'
import type { TrafficRoute } from '@/services/bindings'

export const DAY_FORMAT = 'YYYY-MM-DD'

const timePresets = [
  { startTime: '00:00', endTime: '23:59', label: 'connections.history.allDay' },
  {
    startTime: '00:00',
    endTime: '11:59',
    label: 'connections.history.morning',
  },
  {
    startTime: '12:00',
    endTime: '23:59',
    label: 'connections.history.afternoon',
  },
] as const

export interface TrafficRange {
  date: string
  startTime: string
  endTime: string
}

interface Props {
  range: TrafficRange
  onRangeChange: (range: TrafficRange) => void
  route: TrafficRoute
  onRouteChange: (route: TrafficRoute) => void
  application: string | null
  onClearApplication: () => void
}

const groupStyle = 'inline-flex items-center gap-0.5 rounded-md bg-muted p-0.5'

export function TrafficHistoryFilters({
  range,
  onRangeChange,
  route,
  onRouteChange,
  application,
  onClearApplication,
}: Props) {
  const { t, i18n } = useTranslation()
  const { palette } = useTheme()
  const today = dayjs()
  const todayKey = today.format(DAY_FORMAT)
  const yesterdayKey = today.subtract(1, 'day').format(DAY_FORMAT)
  const [calendarOpen, setCalendarOpen] = useState(false)
  const [draft, setDraft] = useState<Pick<
    TrafficRange,
    'startTime' | 'endTime'
  > | null>(null)
  const { date, startTime, endTime } = range
  const activePreset = timePresets.find(
    (entry) => entry.startTime === startTime && entry.endTime === endTime,
  )
  const customTimesValid =
    !!draft?.startTime && !!draft.endTime && draft.startTime <= draft.endTime
  const locale =
    i18n.language === 'zhtw' ? zhTW : i18n.language === 'zh' ? zhCN : undefined
  const changeDate = (value: string) => onRangeChange({ ...range, date: value })
  const timeRangeLabel = `${startTime}–${endTime}`

  return (
    <div className="space-y-2 rounded-lg border border-border bg-background p-3 text-foreground">
      <div className="flex flex-wrap items-center gap-2">
        <Popover open={calendarOpen} onOpenChange={setCalendarOpen}>
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              size="sm"
              aria-label={`${t('connections.history.date')} ${date}`}
            >
              <CalendarDays /> {date}
            </Button>
          </PopoverTrigger>
          <PopoverContent
            align="start"
            data-traffic-theme={palette.mode}
            className="z-[1400] w-auto p-0"
          >
            <Calendar
              mode="single"
              required
              selected={dayjs(date).toDate()}
              defaultMonth={dayjs(date).toDate()}
              disabled={{ after: today.endOf('day').toDate() }}
              locale={locale}
              onSelect={(value) => {
                changeDate(dayjs(value).format(DAY_FORMAT))
                setCalendarOpen(false)
              }}
            />
          </PopoverContent>
        </Popover>
        <div
          className={groupStyle}
          role="group"
          aria-label={t('connections.history.date')}
        >
          <Button
            variant={date === todayKey ? 'outline' : 'ghost'}
            size="sm"
            aria-pressed={date === todayKey}
            onClick={() => changeDate(todayKey)}
          >
            {t('connections.history.today')}
          </Button>
          <Button
            variant={date === yesterdayKey ? 'outline' : 'ghost'}
            size="sm"
            aria-pressed={date === yesterdayKey}
            onClick={() => changeDate(yesterdayKey)}
          >
            {t('connections.history.yesterday')}
          </Button>
        </div>
        <div
          className={groupStyle}
          role="group"
          aria-label={t('connections.history.route')}
        >
          <Button
            variant={route === 'proxy' ? 'outline' : 'ghost'}
            size="sm"
            aria-pressed={route === 'proxy'}
            onClick={() => onRouteChange('proxy')}
          >
            {t('connections.history.proxy')}
          </Button>
          <Button
            variant={route === 'direct' ? 'outline' : 'ghost'}
            size="sm"
            aria-pressed={route === 'direct'}
            onClick={() => onRouteChange('direct')}
          >
            {t('connections.history.direct')}
          </Button>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <div
          className={groupStyle}
          role="group"
          aria-label={t('connections.history.timeRange')}
        >
          {timePresets.map((preset) => (
            <Button
              key={preset.label}
              variant={activePreset === preset ? 'outline' : 'ghost'}
              size="sm"
              aria-pressed={activePreset === preset}
              onClick={() =>
                onRangeChange({
                  date,
                  startTime: preset.startTime,
                  endTime: preset.endTime,
                })
              }
            >
              {t(preset.label)}
            </Button>
          ))}
        </div>
        <Popover
          open={draft !== null}
          onOpenChange={(open) =>
            setDraft(open ? { startTime, endTime } : null)
          }
        >
          <PopoverTrigger asChild>
            <Button
              variant="outline"
              size="sm"
              aria-label={`${t('connections.history.customTime')} ${timeRangeLabel}`}
            >
              <Clock3 />{' '}
              {activePreset
                ? t('connections.history.customTime')
                : timeRangeLabel}
            </Button>
          </PopoverTrigger>
          {draft && (
            <PopoverContent
              align="start"
              data-traffic-theme={palette.mode}
              className="z-[1400] w-72"
            >
              <div className="grid grid-cols-2 gap-2">
                <label className="grid gap-1.5 text-sm">
                  {t('connections.history.startTime')}
                  <Input
                    type="time"
                    value={draft.startTime}
                    onChange={(event) =>
                      setDraft({ ...draft, startTime: event.target.value })
                    }
                  />
                </label>
                <label className="grid gap-1.5 text-sm">
                  {t('connections.history.endTime')}
                  <Input
                    type="time"
                    value={draft.endTime}
                    onChange={(event) =>
                      setDraft({ ...draft, endTime: event.target.value })
                    }
                  />
                </label>
              </div>
              {!customTimesValid && (
                <p role="status" className="mt-2 text-xs text-destructive">
                  {t('connections.history.invalidRange')}
                </p>
              )}
              <div className="mt-3 flex justify-end gap-2">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setDraft(null)}
                >
                  {t('shared.actions.cancel')}
                </Button>
                <Button
                  size="sm"
                  disabled={!customTimesValid}
                  onClick={() => {
                    onRangeChange({ date, ...draft })
                    setDraft(null)
                  }}
                >
                  {t('shared.actions.confirm')}
                </Button>
              </div>
            </PopoverContent>
          )}
        </Popover>
        {application !== null && (
          <Button
            variant="outline"
            size="sm"
            onClick={onClearApplication}
            className="max-w-full"
          >
            <span className="truncate">
              {t('connections.history.clearApplication', {
                application: application || t('connections.history.unknown'),
              })}
            </span>
            <X />
          </Button>
        )}
      </div>
    </div>
  )
}
