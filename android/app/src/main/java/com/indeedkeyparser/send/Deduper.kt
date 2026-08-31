package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry

class Deduper(private val windowMs: Long = 30_000) {
    private var last: Pair<Entry, Long>? = null

    fun shouldSend(entry: Entry, nowMs: Long): Boolean {
        val prev = last
        if (prev != null && prev.first == entry && nowMs - prev.second < windowMs) {
            return false
        }
        last = entry to nowMs
        return true
    }
}
