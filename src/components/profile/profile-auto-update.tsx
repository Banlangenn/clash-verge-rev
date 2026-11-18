import {
  Box,
  Button,
  FormControlLabel,
  InputAdornment,
  Stack,
  TextField,
} from '@mui/material'
import { useLockFn } from 'ahooks'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, Switch } from '@/components/base'
import { patchProfile } from '@/services/cmds'
import { mutate } from '@/services/mutate'
import { showNotice } from '@/services/notice-service'

const DEFAULT_UPDATE_INTERVAL = 1440
const UPDATE_INTERVAL_PRESETS = [60, 360, 720, DEFAULT_UPDATE_INTERVAL, 4320]

interface Props {
  profileId: string
  option: IProfileOption | undefined
  onSaved: () => Promise<void>
}

export function ProfileAutoUpdate({ profileId, option, onSaved }: Props) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const interval = option?.update_interval ?? 0
  const allowed = option?.allow_auto_update ?? true
  const enabled = allowed && interval > 0
  const draftInterval = draft?.trim() ? Number(draft) : NaN
  const valid = Number.isSafeInteger(draftInterval) && draftInterval >= 0
  const intervalLabel = t('profiles.modals.profileForm.fields.updateInterval')
  const autoUpdateLabel = t(
    'profiles.modals.profileForm.fields.allowAutoUpdate',
  )
  const minuteLabel = t('shared.units.minutes')

  const save = useLockFn(
    async (allowAutoUpdate: boolean, minutes: number, closeDialog: boolean) => {
      setSaving(true)
      try {
        const result = await mutate(
          () =>
            patchProfile(profileId, {
              option: {
                ...option,
                allow_auto_update: allowAutoUpdate,
                update_interval: minutes,
              },
            }),
          { id: `patch-profile:${profileId}`, errorNotice: false },
        )
        if (!result.ok) return
        if (closeDialog) setDraft(null)
        await onSaved()
      } catch (error) {
        showNotice.error('profiles.modals.profileForm.errors.saveFailed', error)
      } finally {
        setSaving(false)
      }
    },
  )

  return (
    <Box onClick={(event) => event.stopPropagation()}>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          flexWrap: 'wrap',
          gap: 0.5,
          mb: 0.5,
        }}
      >
        <FormControlLabel
          sx={{ ml: 0, mr: 0 }}
          slotProps={{ typography: { variant: 'body2' } }}
          label={autoUpdateLabel}
          control={
            <Switch
              checked={enabled}
              disabled={saving}
              onChange={(_event, checked) =>
                save(
                  checked,
                  checked && !interval ? DEFAULT_UPDATE_INTERVAL : interval,
                  false,
                )
              }
            />
          }
        />
        <Button
          size="small"
          disabled={saving}
          onClick={() => setDraft(String(interval || DEFAULT_UPDATE_INTERVAL))}
        >
          {intervalLabel}
          {interval > 0 ? ` · ${interval} ${minuteLabel}` : ''}
        </Button>
      </Box>
      <BaseDialog
        open={draft !== null}
        title={intervalLabel}
        okBtn={t('shared.actions.save')}
        cancelBtn={t('shared.actions.cancel')}
        loading={saving}
        disableOk={!valid}
        onClose={saving ? undefined : () => setDraft(null)}
        onCancel={() => {
          if (!saving) setDraft(null)
        }}
        onOk={() => {
          if (valid) return save(allowed, draftInterval, true)
        }}
      >
        <Stack spacing={2} sx={{ py: 1 }}>
          <Stack
            direction="row"
            spacing={1}
            useFlexGap
            sx={{ flexWrap: 'wrap' }}
          >
            {UPDATE_INTERVAL_PRESETS.map((minutes) => (
              <Button
                key={minutes}
                size="small"
                variant={draftInterval === minutes ? 'contained' : 'outlined'}
                aria-pressed={draftInterval === minutes}
                disabled={saving}
                onClick={() => setDraft(String(minutes))}
              >
                {minutes <= DEFAULT_UPDATE_INTERVAL
                  ? t('profiles.modals.profileForm.presets.hours', {
                      count: minutes / 60,
                    })
                  : t('profiles.modals.profileForm.presets.days', {
                      count: minutes / DEFAULT_UPDATE_INTERVAL,
                    })}
              </Button>
            ))}
          </Stack>
          <TextField
            label={intervalLabel}
            type="number"
            size="small"
            value={draft ?? ''}
            disabled={saving}
            onChange={(event) => setDraft(event.target.value)}
            helperText={
              valid &&
              draftInterval > 0 &&
              draftInterval < DEFAULT_UPDATE_INTERVAL
                ? t('profiles.modals.profileForm.warnings.frequentUpdate', {
                    minutes: DEFAULT_UPDATE_INTERVAL,
                  })
                : undefined
            }
            slotProps={{
              htmlInput: { min: 0, step: 1 },
              formHelperText: { sx: { color: 'warning.main' } },
              input: {
                endAdornment: (
                  <InputAdornment position="end">{minuteLabel}</InputAdornment>
                ),
              },
            }}
          />
        </Stack>
      </BaseDialog>
    </Box>
  )
}
