package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeduperTest {
    private val e = Entry("Corp", "123456")

    @Test
    fun sends_first_occurrence() {
        assertTrue(Deduper().shouldSend(e, 0))
    }

    @Test
    fun suppresses_same_within_window() {
        val d = Deduper(windowMs = 30_000)
        assertTrue(d.shouldSend(e, 0))
        assertFalse(d.shouldSend(e, 10_000))
    }

    @Test
    fun sends_again_after_window() {
        val d = Deduper(windowMs = 30_000)
        assertTrue(d.shouldSend(e, 0))
        assertTrue(d.shouldSend(e, 40_000))
    }

    @Test
    fun different_entry_sends() {
        val d = Deduper()
        assertTrue(d.shouldSend(e, 0))
        assertTrue(d.shouldSend(Entry("Bank", "123456"), 1000))
    }
}
