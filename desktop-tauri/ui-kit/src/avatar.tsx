import { Avatar as BaseAvatar } from '@base-ui/react/avatar'
import { cn } from './lib/cn'

function Avatar({ className, ...props }: BaseAvatar.Root.Props) {
  return <BaseAvatar.Root data-slot='avatar' className={state => cn(
    'relative inline-flex size-8 shrink-0 overflow-hidden rounded-full text-foreground',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function AvatarImage({ className, ...props }: BaseAvatar.Image.Props) {
  return <BaseAvatar.Image data-slot='avatar-image' className={state => cn(
    'absolute inset-0 block size-full object-cover data-loading:invisible data-error:invisible',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

function AvatarFallback({ className, ...props }: BaseAvatar.Fallback.Props) {
  return <BaseAvatar.Fallback data-slot='avatar-fallback' className={state => cn(
    'absolute inset-0 flex items-center justify-center bg-control text-[12px] font-semibold text-subtle',
    typeof className === 'function' ? className(state) : className,
  )} {...props} />
}

export { Avatar, AvatarImage, AvatarFallback }
