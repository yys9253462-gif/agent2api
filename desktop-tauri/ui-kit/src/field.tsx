import * as React from 'react'
import { Field as BaseField } from '@base-ui/react/field'
import { Fieldset as BaseFieldset } from '@base-ui/react/fieldset'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from './lib/cn'

const fieldVariants = cva('min-w-0', {
  variants: {
    orientation: {
      vertical: 'flex flex-col gap-1.5',
      horizontal: 'flex flex-wrap items-center gap-2.5',
      responsive: 'flex flex-col gap-1.5 min-[768px]:flex-row min-[768px]:items-center min-[768px]:gap-2.5',
    },
  },
  defaultVariants: { orientation: 'vertical' },
})

type FieldProps = BaseField.Root.Props & VariantProps<typeof fieldVariants>

function Field({ className, orientation, ...props }: FieldProps) {
  return <BaseField.Root data-slot='field' className={state => cn(
    fieldVariants({ orientation }),
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function FieldGroup({ className, orientation, ...props }: React.ComponentProps<'div'> & VariantProps<typeof fieldVariants>) {
  return <div data-slot='field-group' className={cn(orientation ? fieldVariants({ orientation }) : 'flex flex-col gap-3', className)} {...props} />
}

function FieldSet({ className, ...props }: BaseFieldset.Root.Props) {
  return <BaseFieldset.Root data-slot='field-set' className={state => cn(
    'm-0 min-w-0 border-0 p-0',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function FieldLegend({ className, ...props }: BaseFieldset.Legend.Props) {
  return <BaseFieldset.Legend data-slot='field-legend' className={state => cn(
    'mb-2 text-[13px] font-semibold',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function FieldLabel({ className, ...props }: BaseField.Label.Props) {
  return <BaseField.Label data-slot='field-label' className={state => cn(
    'flex items-center gap-[6px] text-[11.5px] text-muted-foreground',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function FieldContent({ className, ...props }: React.ComponentProps<'div'>) {
  return <div data-slot='field-content' className={cn('min-w-0 flex-1', className)} {...props} />
}

function FieldDescription({ className, ...props }: BaseField.Description.Props) {
  return <BaseField.Description data-slot='field-description' className={state => cn(
    'm-0 text-[11.5px] leading-relaxed text-subtle',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function FieldError({ className, ...props }: BaseField.Error.Props) {
  return <BaseField.Error data-slot='field-error' role='alert' className={state => cn(
    'text-[12px] text-destructive',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

export { Field, FieldGroup, FieldSet, FieldLegend, FieldLabel, FieldContent, FieldDescription, FieldError, type FieldProps }
