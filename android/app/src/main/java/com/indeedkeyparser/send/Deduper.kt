package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry

/** Send-on-change: forwards an entry only when it differs from the last one that
 *  was accepted, so a code that stays on screen across many polls is sent once. */
class Deduper {
    private var last: Entry? = null

    fun shouldSend(entry: Entry): Boolean {
        if (entry == last) return false
        last = entry
        return true
    }
}
