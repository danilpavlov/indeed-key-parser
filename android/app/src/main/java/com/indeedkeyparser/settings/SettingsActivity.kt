package com.indeedkeyparser.settings

import android.content.Intent
import android.os.Bundle
import android.provider.Settings as AndroidSettings
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.RadioButton
import android.widget.RadioGroup
import androidx.appcompat.app.AppCompatActivity
import com.indeedkeyparser.R

class SettingsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)
        val settings = Settings(this)

        val modeGroup = findViewById<RadioGroup>(R.id.modeGroup)
        val modeDirect = findViewById<RadioButton>(R.id.modeDirect)
        val modeRemote = findViewById<RadioButton>(R.id.modeRemote)
        val directBox = findViewById<View>(R.id.directBox)
        val remoteBox = findViewById<View>(R.id.remoteBox)
        val port = findViewById<EditText>(R.id.directPort).apply { setText(settings.directPort.toString()) }
        val url = findViewById<EditText>(R.id.webhookUrl).apply { setText(settings.webhookUrl) }
        val secret = findViewById<EditText>(R.id.secret).apply { setText(settings.secret) }

        fun showBoxesFor(mode: WebhookMode) {
            directBox.visibility = if (mode == WebhookMode.DIRECT) View.VISIBLE else View.GONE
            remoteBox.visibility = if (mode == WebhookMode.REMOTE) View.VISIBLE else View.GONE
        }

        (if (settings.mode == WebhookMode.DIRECT) modeDirect else modeRemote).isChecked = true
        showBoxesFor(settings.mode)
        modeGroup.setOnCheckedChangeListener { _, checkedId ->
            showBoxesFor(if (checkedId == R.id.modeDirect) WebhookMode.DIRECT else WebhookMode.REMOTE)
        }

        findViewById<Button>(R.id.save).setOnClickListener {
            settings.mode = if (modeDirect.isChecked) WebhookMode.DIRECT else WebhookMode.REMOTE
            settings.directPort = port.text.toString().trim().toIntOrNull() ?: 8080
            settings.webhookUrl = url.text.toString().trim()
            settings.secret = secret.text.toString().trim()
        }
        findViewById<Button>(R.id.enableAccessibility).setOnClickListener {
            startActivity(Intent(AndroidSettings.ACTION_ACCESSIBILITY_SETTINGS))
        }
    }
}
