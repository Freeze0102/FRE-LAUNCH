import type { AppUpdate } from '@/providers/app-update'

type Ref<T> = { value: T }
type State = {
	availableUpdate: Ref<AppUpdate | null>
	finishedDownloading: Ref<boolean>
	downloading: Ref<boolean>
	restarting: Ref<boolean>
	updatesEnabled: Ref<boolean>
	metered: Ref<boolean>
	updateSize: Ref<number | null>
	progress: Ref<number>
}
type Options = {
	state: State
	enabled: () => Promise<boolean>
	check: () => Promise<AppUpdate | null>
	metered: () => Promise<boolean>
	size: (rid: number) => Promise<number | null>
	download: (rid: number) => Promise<unknown>
	subscribe: (version: string) => Promise<() => void>
	release: (rid: number) => Promise<unknown>
	ready: (update: AppUpdate) => void
	available: (update: AppUpdate) => void
	restart: (enabled: boolean) => Promise<unknown>
	close: () => Promise<unknown>
	error: (error: unknown) => void
	warn: (error: unknown) => void
	intervalMs?: number
}

// One owner for polling, staging and restart state. Installation is never
// triggered by the poller or by download completion.
export function createAppUpdater(options: Options) {
	const state = options.state
	let stopped = false
	let checking = false
	let started = false
	let timer: ReturnType<typeof setTimeout> | undefined
	let unsubscribe: (() => void) | undefined
	let promptedVersion: string | undefined

	async function download(fromCheck = false) {
		const update = state.availableUpdate.value
		if (stopped || (checking && !fromCheck) || !update || state.downloading.value || state.finishedDownloading.value) return
		state.downloading.value = true
		state.progress.value = 0
		try {
			// Listen first: a cached or small installer can finish immediately.
			unsubscribe = await options.subscribe(update.version)
			if (stopped) return
			await options.download(update.rid)
			if (stopped) return
			state.progress.value = 1
			state.finishedDownloading.value = true
			options.ready(update)
		} catch (error) {
			state.progress.value = 0
			if (!stopped) options.error(error)
		} finally {
			unsubscribe?.()
			unsubscribe = undefined
			state.downloading.value = false
		}
	}

	async function check() {
		if (stopped || checking || state.downloading.value || state.finishedDownloading.value) return
		checking = true
		try {
			const update = await options.check()
			if (!update) return
			if (stopped) { await options.release(update.rid); return }
			const previous = state.availableUpdate.value
			state.availableUpdate.value = update
			if (previous && previous.rid !== update.rid) await options.release(previous.rid).catch(options.warn)
			state.updateSize.value = null
			void options.size(update.rid).then((size) => {
				if (!stopped && state.availableUpdate.value?.rid === update.rid) state.updateSize.value = size
			}).catch(options.warn)
			state.metered.value = await options.metered().catch(() => true)
			if (stopped) return
			if (state.metered.value) {
				if (promptedVersion !== update.version) options.available(update)
				promptedVersion = update.version
			} else await download(true)
		} catch (error) {
			if (!stopped) options.warn(error)
		} finally {
			checking = false
		}
	}

	async function poll() {
		await check()
		if (!stopped) timer = setTimeout(poll, options.intervalMs ?? 15 * 60 * 1000)
	}

	async function start() {
		if (started || stopped) return
		started = true
		try {
			state.updatesEnabled.value = await options.enabled()
			if (!stopped && state.updatesEnabled.value) await poll()
		} catch (error) {
			options.warn(error)
		}
	}

	async function install() {
		if (stopped || !state.finishedDownloading.value || state.restarting.value) return
		state.restarting.value = true
		try {
			await options.restart(true)
			await options.close()
		} catch (error) {
			state.restarting.value = false
			await options.restart(false).catch(options.warn)
			options.error(error)
		}
	}

	function stop() {
		stopped = true
		if (timer !== undefined) clearTimeout(timer)
		unsubscribe?.()
		unsubscribe = undefined
	}

	return { start, stop, check, download: () => download(), install }
}
