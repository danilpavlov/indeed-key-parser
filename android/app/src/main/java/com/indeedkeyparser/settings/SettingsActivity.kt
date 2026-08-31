package com.indeedkeyparser.settings

import android.content.Intent
import android.os.Bundle
import android.provider.Settings as AndroidSettings
import android.widget.Button
import android.widget.EditText
import androidx.appcompat.app.AppCompatActivity
import com.indeedkeyparser.R

class SettingsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        val settings = Settings(this)
        val url = findViewById<EditText>(R.id.webhookUrl).apply { setText(settings.webhookUrl) }
        val secret = findViewById<EditText>(R.id.secret).apply { setText(settings.secret) }
        findViewById<Button>(R.id.save).setOnClickListener {
            settings.webhookUrl = url.text.toString().trim()
            settings.secret = secret.text.toString().trim()
        }
        findViewById<Button>(R.id.enableAccessibility).setOnClickListener {
            startActivity(Intent(AndroidSettings.ACTION_ACCESSIBILITY_SETTINGS))
        }
    }
}
