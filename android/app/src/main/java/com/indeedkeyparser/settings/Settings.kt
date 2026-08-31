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
}
