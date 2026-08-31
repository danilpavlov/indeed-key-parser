package com.indeedkeyparser

import android.accessibilityservice.AccessibilityService
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import com.indeedkeyparser.parse.UiNode
import com.indeedkeyparser.parse.extractEntry
import com.indeedkeyparser.send.Deduper
import com.indeedkeyparser.send.WebhookSender

class IndeedCodeAccessibilityService : AccessibilityService() {
    private val deduper = Deduper()

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        val root = rootInActiveWindow ?: return
        val entry = extractEntry(toUiNode(root)) ?: return
        if (deduper.shouldSend(entry, System.currentTimeMillis())) {
            WebhookSender.enqueue(applicationContext, entry)
        }
    }

    override fun onInterrupt() {}

    private fun toUiNode(info: AccessibilityNodeInfo?): UiNode {
        if (info == null) return UiNode(null)
        val children = (0 until info.childCount).mapNotNull { i ->
            info.getChild(i)?.let { toUiNode(it) }
        }
        return UiNode(info.text?.toString(), children)
    }
}
