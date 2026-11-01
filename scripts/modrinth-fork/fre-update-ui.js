const appUpdateDownload = { progress: appUpdateState.progress, version: ref() }
const { restarting } = appUpdateState
const updatePopupMessages = defineMessages({
	ready: { id: 'app.update-popup.download-complete', defaultMessage: 'Update ready' },
	available: { id: 'app.update-popup.title', defaultMessage: 'Update available' },
	downloadedBody: {
		id: 'app.update-popup.body.download-complete',
		defaultMessage: 'FRE LAUNCH v{version} is ready. Restart to update, or keep playing and it will install when you close the launcher.',
	},
	meteredBody: {
		id: 'app.update-popup.body.metered',
		defaultMessage: 'FRE LAUNCH v{version} is available. Download it when you are ready to use data on this metered connection.',
	},
	restart: { id: 'app.update-popup.reload', defaultMessage: 'Restart and update' },
	download: { id: 'fre-launch.update.download', defaultMessage: 'Download update' },
	changelog: { id: 'app.update-popup.changelog', defaultMessage: 'Changelog' },
})

function showFreUpdatePrompt(update, ready) {
	addPopupNotification({
		contentType: 'standard',
		title: formatMessage(ready ? updatePopupMessages.ready : updatePopupMessages.available),
		text: formatMessage(ready ? updatePopupMessages.downloadedBody : updatePopupMessages.meteredBody,
			{ version: update.version }),
		type: ready ? 'success' : 'info',
		autoCloseMs: null,
		buttons: [
			{
				label: formatMessage(ready ? updatePopupMessages.restart : updatePopupMessages.download),
				action: () => ready ? installAvailableAppUpdate() : downloadAvailableAppUpdate(),
				color: 'brand',
			},
			{ label: formatMessage(updatePopupMessages.changelog), action: openAppUpdateChangelog, keepOpen: true },
		],
	})
}

const freAppUpdater = createAppUpdater({
	state: appUpdateState,
	enabled: areUpdatesEnabled,
	check: () => invoke('plugin:updater|check', { timeout: 30000 }),
	metered: isNetworkMetered,
	size: getUpdateSize,
	download: enqueueUpdateForInstallation,
	subscribe: (version) => subscribeToDownloadProgress(appEvents, appUpdateDownload, version),
	release: (rid) => invoke('plugin:resources|close', { rid }),
	ready: (update) => showFreUpdatePrompt(update, true),
	available: (update) => showFreUpdatePrompt(update, false),
	restart: setRestartAfterPendingUpdate,
	close: handleClose,
	error: handleError,
	warn: (error) => console.warn('FRE LAUNCH update:', error),
})

watch(stateInitialized, (ready) => { if (ready) void freAppUpdater.start() })
setAppUpdateActions({
	download: freAppUpdater.download,
	install: freAppUpdater.install,
	changelog: () => openUrl('__FRE_RELEASES_URL__'),
})
