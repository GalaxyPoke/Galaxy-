import type { AbstractPopupNotificationManager, AbstractWebNotificationManager } from '@modrinth/ui'

import { setupFilePickerProvider } from './setup/file-picker'
import { setupImageViewerEditorProvider } from './setup/image-viewer-editor'
import { setupTagsProvider } from './setup/tags'

/**
 * Galaxy宝可梦 (slimming phase 1): the creation-flow (`./setup/creation-modal`), global file-drop
 * (`./setup/file-drop`) and launcher-import (`./setup/instance-import`) providers are no longer
 * installed. Their only consumers were the instance creation / import / drag-and-drop modals that
 * App.vue no longer mounts. The modules stay on disk; to restore, call
 * `setupFileDropProvider()`, `setupInstanceImportProvider(notificationManager)` and
 * `setupCreationModal(notificationManager, popupNotificationManager)` here again and return
 * `fileDrop` plus the creation-modal helpers.
 */
export function setupProviders(
    notificationManager: AbstractWebNotificationManager,
    _popupNotificationManager: AbstractPopupNotificationManager,
    stateInitialization: Promise<void>,
) {
    setupTagsProvider(notificationManager, stateInitialization)
    const filePicker = setupFilePickerProvider()
    setupImageViewerEditorProvider()

    return {
        ...filePicker,
    }
}
