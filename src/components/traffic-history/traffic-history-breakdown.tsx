import ChevronRightOutlined from '@mui/icons-material/ChevronRightOutlined'
import {
  Box,
  Button,
  LinearProgress,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  ToggleButton,
  ToggleButtonGroup,
  Typography,
} from '@mui/material'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button as ShadcnButton } from '@/components/ui/button'
import type { TrafficBreakdown, TrafficDimension } from '@/services/bindings'

import { TrafficHistoryShare } from './traffic-history-share'
import {
  formatTraffic,
  formatTrafficCost,
  formatTrafficPeriod,
  type TrafficPeriod,
} from './traffic-value'

const dimensions = {
  application: 'connections.history.application',
  destination: 'connections.history.destination',
  node: 'connections.history.node',
} as const

export interface TrafficGrouping {
  dimension: TrafficDimension
  application: string | null
}

interface Props {
  rows: TrafficBreakdown[]
  costRows: TrafficBreakdown[]
  total: number
  grouping: TrafficGrouping
  onGroupingChange: (grouping: TrafficGrouping) => void
  onClearApplication: () => void
  focusedPeriod: TrafficPeriod | null
}

export function TrafficHistoryBreakdown({
  rows,
  costRows,
  total,
  grouping,
  onGroupingChange,
  onClearApplication,
  focusedPeriod,
}: Props) {
  const { t } = useTranslation()
  const { dimension, application } = grouping
  const [sortByCost, setSortByCost] = useState(false)
  const hasCosts = costRows.length > 0
  const costSorted = hasCosts && sortByCost
  const rankedRows = costSorted ? costRows : rows
  const unknownLabel = t(
    dimension === 'application'
      ? 'connections.history.unknownApplication'
      : 'connections.history.unknown',
  )

  return (
    <Paper variant="outlined" sx={{ borderRadius: 2, overflow: 'hidden' }}>
      <Stack
        direction="row"
        spacing={1.25}
        useFlexGap
        sx={{
          p: 1.5,
          flexWrap: 'wrap',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <Typography
            component="h2"
            variant="subtitle1"
            sx={{ fontWeight: 700, overflowWrap: 'anywhere' }}
          >
            {application === null
              ? t('connections.history.breakdown')
              : application || t('connections.history.unknownApplication')}
          </Typography>
          {focusedPeriod && (
            <span className="text-xs text-muted-foreground">
              {formatTrafficPeriod(focusedPeriod)}
            </span>
          )}
          {application !== null && (
            <ShadcnButton
              variant="outline"
              size="sm"
              onClick={onClearApplication}
            >
              {t('connections.history.backToApplications')}
            </ShadcnButton>
          )}
        </div>
        <ToggleButtonGroup
          size="small"
          exclusive
          color="primary"
          aria-label={t('connections.history.groupBy')}
          value={dimension}
          onChange={(_event, value: TrafficDimension | null) => {
            if (value !== null)
              onGroupingChange({
                dimension: value,
                application: value === 'application' ? null : application,
              })
          }}
          sx={{
            '& .MuiToggleButton-root': { px: 1.25, textTransform: 'none' },
          }}
        >
          {Object.entries(dimensions).map(([value, label]) => (
            <ToggleButton key={value} value={value}>
              {t(label)}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>
      </Stack>
      {rows.length > 0 && (
        <Box
          sx={{
            display: 'grid',
            gridTemplateColumns: {
              xs: 'minmax(0, 1fr)',
              md: 'minmax(0, 1fr) minmax(210px, 32%)',
            },
            alignItems: 'start',
          }}
        >
          <TableContainer>
            <Table
              size="small"
              sx={{
                tableLayout: 'fixed',
                minWidth: hasCosts ? 405 : undefined,
                '& .MuiTableCell-root': { px: 1.5 },
              }}
            >
              <TableHead sx={{ bgcolor: 'action.hover' }}>
                <TableRow>
                  <TableCell sx={{ width: hasCosts ? '30%' : '45%' }}>
                    {t(dimensions[dimension])}
                  </TableCell>
                  <TableCell align="right">
                    {t('connections.history.upload')}
                  </TableCell>
                  <TableCell align="right">
                    {t('connections.history.download')}
                  </TableCell>
                  <TableCell
                    align="right"
                    sortDirection={hasCosts && !costSorted ? 'desc' : false}
                  >
                    {hasCosts ? (
                      <ShadcnButton
                        size="sm"
                        variant="ghost"
                        className="h-auto p-0 text-xs"
                        aria-pressed={!costSorted}
                        onClick={() => setSortByCost(false)}
                      >
                        {t('connections.history.total')}
                        {!costSorted && <span aria-hidden="true">↓</span>}
                      </ShadcnButton>
                    ) : (
                      t('connections.history.total')
                    )}
                  </TableCell>
                  {hasCosts && (
                    <TableCell
                      align="right"
                      sortDirection={costSorted ? 'desc' : false}
                    >
                      <ShadcnButton
                        size="sm"
                        variant="ghost"
                        className="h-auto whitespace-normal p-0 text-xs"
                        aria-pressed={costSorted}
                        onClick={() => setSortByCost(true)}
                      >
                        {t('connections.history.estimatedCost')}
                        {costSorted && <span aria-hidden="true">↓</span>}
                      </ShadcnButton>
                    </TableCell>
                  )}
                </TableRow>
              </TableHead>
              <TableBody sx={{ '& tr:last-child td': { borderBottom: 0 } }}>
                {rankedRows.map((row) => {
                  const amount = row.upload + row.download
                  const name = row.name || unknownLabel
                  return (
                    <TableRow
                      hover
                      key={row.name}
                      sx={{ fontVariantNumeric: 'tabular-nums' }}
                    >
                      <TableCell sx={{ overflowWrap: 'anywhere' }}>
                        {dimension === 'application' ? (
                          <Button
                            size="small"
                            endIcon={<ChevronRightOutlined />}
                            onClick={() =>
                              onGroupingChange({
                                dimension: 'destination',
                                application: row.name,
                              })
                            }
                            sx={{
                              px: 0,
                              minWidth: 0,
                              maxWidth: '100%',
                              textTransform: 'none',
                              textAlign: 'left',
                            }}
                          >
                            {name}
                          </Button>
                        ) : (
                          name
                        )}
                      </TableCell>
                      <TableCell align="right">
                        {formatTraffic(row.upload)}
                      </TableCell>
                      <TableCell align="right">
                        {formatTraffic(row.download)}
                      </TableCell>
                      <TableCell align="right">
                        <Box sx={{ fontWeight: 700 }}>
                          {formatTraffic(amount)}
                        </Box>
                        <LinearProgress
                          variant="determinate"
                          value={total > 0 ? (amount / total) * 100 : 0}
                          aria-label={name}
                          sx={{
                            mt: 0.5,
                            height: 3,
                            borderRadius: 1,
                            '& .MuiLinearProgress-bar': { borderRadius: 1 },
                          }}
                        />
                      </TableCell>
                      {hasCosts && (
                        <TableCell align="right">
                          {row.estimatedCost === null
                            ? null
                            : formatTrafficCost(row.estimatedCost)}
                          {row.partialCost && (
                            <div className="text-xs text-muted-foreground">
                              {t('connections.history.partialCost')}
                            </div>
                          )}
                        </TableCell>
                      )}
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </TableContainer>
          {total > 0 && (
            <TrafficHistoryShare
              rows={rows}
              total={total}
              unknownLabel={unknownLabel}
            />
          )}
        </Box>
      )}
    </Paper>
  )
}
