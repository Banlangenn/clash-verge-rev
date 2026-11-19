import dayjs from 'dayjs'

import parseTraffic from '@/utils/parse-traffic'

export interface TrafficPeriod {
  from: number
  until: number
}

export function formatTrafficPeriod({ from, until }: TrafficPeriod) {
  const start = dayjs.unix(from)
  const end = dayjs.unix(Math.max(from, until - 60))
  if (start.isSame(end, 'minute')) return start.format('HH:mm')
  const format = start.isSame(end, 'day') ? 'HH:mm' : 'MM-DD HH:mm'
  return `${start.format(format)}–${end.format(format)}`
}

export function formatTraffic(amount: number) {
  return parseTraffic(amount).join(' ')
}

const shareFormatter = new Intl.NumberFormat(undefined, {
  style: 'percent',
  maximumFractionDigits: 1,
})

export function formatTrafficShare(fraction: number) {
  return shareFormatter.format(fraction)
}

const costFormatter = new Intl.NumberFormat('zh-CN', {
  style: 'currency',
  currency: 'CNY',
  maximumFractionDigits: 4,
})

export function formatTrafficCost(amount: number) {
  return amount > 0 && amount < 0.0001
    ? `< ${costFormatter.format(0.0001)}`
    : costFormatter.format(amount)
}
