import parseTraffic from '@/utils/parse-traffic'

export function formatTraffic(amount: number) {
  return parseTraffic(amount).join(' ')
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
