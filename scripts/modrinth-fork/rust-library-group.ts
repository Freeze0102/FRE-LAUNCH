// Rust profiles use their own native backend but share Java's library layout.
// These IDs match the pinned Modrinth library grouping conventions exactly.
export function rustLibraryGroupId(group: string): string {
  switch (group) {
    case 'Group': return 'group:none'
    case 'None': return 'None:None'
    case 'Loader': return 'Loader:Rust'
    case 'Game version': return 'Game version:26.3'
    case 'Instance type': return 'Instance type:Rust'
    default: return 'group:none'
  }
}

export function rustLibraryGroupLabel(group: string): string {
  if (group === 'Game version') return '26.3'
  if (group === 'Group' || group === 'None') return 'None'
  return 'Rust'
}
