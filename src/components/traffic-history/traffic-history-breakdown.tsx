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
import { useTranslation } from 'react-i18next'

import type { TrafficBreakdown, TrafficDimension } from '@/services/bindings'

import { TrafficHistoryShare } from './traffic-history-share'
import { formatTraffic } from './traffic-value'

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
  total: number
  grouping: TrafficGrouping
  onGroupingChange: (grouping: TrafficGrouping) => void
}

export function TrafficHistoryBreakdown({
  rows,
  total,
  grouping,
  onGroupingChange,
}: Props) {
  const { t } = useTranslation()
  const { dimension, application } = grouping

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
        <Typography component="h2" variant="subtitle1" sx={{ fontWeight: 700 }}>
          {t('connections.history.breakdown')}
        </Typography>
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
              sx={{ tableLayout: 'fixed', '& .MuiTableCell-root': { px: 1.5 } }}
            >
              <TableHead sx={{ bgcolor: 'action.hover' }}>
                <TableRow>
                  <TableCell sx={{ width: '45%' }}>
                    {t(dimensions[dimension])}
                  </TableCell>
                  <TableCell align="right">
                    {t('connections.history.upload')}
                  </TableCell>
                  <TableCell align="right">
                    {t('connections.history.download')}
                  </TableCell>
                  <TableCell align="right">
                    {t('connections.history.total')}
                  </TableCell>
                </TableRow>
              </TableHead>
              <TableBody sx={{ '& tr:last-child td': { borderBottom: 0 } }}>
                {rows.map((row) => {
                  const amount = row.upload + row.download
                  const name = row.name || t('connections.history.unknown')
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
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </TableContainer>
          {total > 0 && <TrafficHistoryShare rows={rows} total={total} />}
        </Box>
      )}
    </Paper>
  )
}
