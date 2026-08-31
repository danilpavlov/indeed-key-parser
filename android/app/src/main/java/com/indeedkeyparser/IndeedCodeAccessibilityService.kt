package com.indeedkeyparser

import android.accessibilityservice.AccessibilityService
import android.os.Handler
import android.os.Looper
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import com.indeedkeyparser.parse.UiNode
import com.indeedkeyparser.parse.extractEntry
import com.indeedkeyparser.send.Deduper
import com.indeedkeyparser.send.WebhookSender

class IndeedCodeAccessibilityService : AccessibilityService() {
    private val deduper = Deduper()
    private val handler = Handler(Looper.getMainLooper())

    // Re-reads the Indeed Key screen every POLL_INTERVAL_MS so a revealed code is
    // forwarded without the user having to leave and re-open the app to trigger a
    // window event. The Deduper keeps it to send-on-change.
    private val poll = object : Runnable {
        override fun run() {
            captureAndSend()
            handler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        handler.postDelayed(poll, POLL_INTERVAL_MS)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        captureAndSend()
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        handler.removeCallbacks(poll)
        super.onDestroy()
    }

    private fun captureAndSend() {
        val root = rootInActiveWindow ?: return
        if (root.packageName?.toString() != INDEED_KEY_PACKAGE) return
        val entry = extractEntry(toUiNode(root)) ?: return
        if (deduper.shouldSend(entry)) {
            WebhookSender.enqueue(applicationContext, entry)
        }
    }

    private fun toUiNode(info: AccessibilityNodeInfo?): UiNode {
        if (info == null) return UiNode(null)
        val children = (0 until info.childCount).mapNotNull { i ->
            info.getChild(i)?.let { toUiNode(it) }
        }
        return UiNode(info.text?.toString(), children, info.viewIdResourceName)
    }

    private companion object {
        const val POLL_INTERVAL_MS = 2_000L
        const val INDEED_KEY_PACKAGE = "com.indeedid.key"
    }
}
