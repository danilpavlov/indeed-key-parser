package com.indeedkeyparser.settings

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

class Settings(context: Context) {
    private val prefs = run {
        val key = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context,
            "settings",
            key,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }

    /** Defaults to REMOTE so an existing install keeps using its saved webhook URL. */
    var mode: WebhookMode
        get() = runCatching { WebhookMode.valueOf(prefs.getString("mode", null) ?: "REMOTE") }
            .getOrDefault(WebhookMode.REMOTE)
        set(v) {
            prefs.edit().putString("mode", v.name).apply()
        }

    /** Loopback port used by DIRECT mode (matches `adb reverse tcp:<port>`). */
    var directPort: Int
        get() = prefs.getInt("direct_port", 8080)
        set(v) {
            prefs.edit().putInt("direct_port", v).apply()
        }

    /** The REMOTE-mode webhook URL. */
    var webhookUrl: String
        get() = prefs.getString("webhook_url", "") ?: ""
        set(v) {
            prefs.edit().putString("webhook_url", v).apply()
        }

    var secret: String
        get() = prefs.getString("secret", "") ?: ""
        set(v) {
            prefs.edit().putString("secret", v).apply()
        }

    /** The URL the sender should POST to for the current mode. */
    val resolvedUrl: String
        get() = WebhookTarget.resolveUrl(mode, directPort, webhookUrl)

    /** Whether delivery needs an active network connection in the current mode. */
    val requiresNetwork: Boolean
        get() = WebhookTarget.requiresNetwork(mode)
}
