package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DeduperTest {
    private val e = Entry("Corp", "123456")

    @Test
    fun sends_first_occurrence() {
        assertTrue(Deduper().shouldSend(e))
    }

    @Test
    fun suppresses_immediate_repeat_of_same_code() {
        val d = Deduper()
        assertTrue(d.shouldSend(e))
        assertFalse(d.shouldSend(e))
        assertFalse(d.shouldSend(e))
    }

    @Test
    fun sends_when_the_code_changes() {
        val d = Deduper()
        assertTrue(d.shouldSend(e))
        assertTrue(d.shouldSend(Entry("Corp", "654321")))
    }

    @Test
    fun sends_again_when_a_previous_code_reappears_after_a_change() {
        val d = Deduper()
        val a = Entry("Corp", "111111")
        val b = Entry("Corp", "222222")
        assertTrue(d.shouldSend(a))
        assertTrue(d.shouldSend(b))
        assertTrue(d.shouldSend(a)) // only the immediately-previous value is suppressed
    }

    @Test
    fun different_account_with_same_code_sends() {
        val d = Deduper()
        assertTrue(d.shouldSend(e))
        assertTrue(d.shouldSend(Entry("Bank", "123456")))
    }
}
