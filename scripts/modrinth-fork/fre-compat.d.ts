import 'ace-builds'

declare module 'ace-builds' {
	export function define(
		name: string,
		dependencies: string[],
		factory: (require: typeof import('ace-builds').require, exports: Record<string, unknown>, module: unknown) => void,
	): void
}
