import { defineStore } from 'pinia'

// Galaxy宝可梦: offline accounts only. The anti-piracy notice and the Microsoft
// auth-error modal are never registered, so every error goes to the error modal.

export const useError = defineStore('errorsStore', {
    state: () => ({
        errorModal: null,
        minecraftAuthErrorModal: null,
        minecraftLaunchErrorHandler: null,
        antiPiracyNoticeModal: null,
    }),
    actions: {
        setErrorModal(ref) {
            this.errorModal = ref
        },
        setMinecraftAuthErrorModal(ref) {
            this.minecraftAuthErrorModal = ref
        },
        setAntiPiracyNoticeModal(ref) {
            this.antiPiracyNoticeModal = ref
        },
        showAntiPiracyNotice() {
            this.antiPiracyNoticeModal?.show()
        },
        setMinecraftLaunchErrorHandler(handler) {
            this.minecraftLaunchErrorHandler = handler
        },
        showError(error, context, closable = true, source = null) {
            if (this.minecraftLaunchErrorHandler?.(error, context)) return
            this.errorModal.show(error, context, closable, source)
        },
    },
})

export const handleSevereError = (err, context) => {
    const error = useError()
    error.showError(err, context)
    console.error(err)
}
