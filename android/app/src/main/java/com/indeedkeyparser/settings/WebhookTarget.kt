package com.indeedkeyparser.settings

/** Where codes are sent. [DIRECT] is the phone plugged into the server host over
 *  USB (adb reverse -> localhost); [REMOTE] is a server the phone reaches over the
 *  network (LAN device or VPS). */
enum class WebhookMode { DIRECT, REMOTE }

object WebhookTarget {
    /** The URL to POST to for the given mode. Direct always hits loopback on the
     *  configured port; remote uses the user-entered URL. */
    fun resolveUrl(mode: WebhookMode, directPort: Int, remoteUrl: String): String =
        when (mode) {
            WebhookMode.DIRECT -> "http://localhost:$directPort/webhook"
            WebhookMode.REMOTE -> remoteUrl.trim()
        }

    /** Direct posts over loopback and needs no network; remote does. Used to pick
     *  the WorkManager network constraint so direct delivery never stalls waiting
     *  for a connection the phone does not need. */
    fun requiresNetwork(mode: WebhookMode): Boolean = mode == WebhookMode.REMOTE
}
