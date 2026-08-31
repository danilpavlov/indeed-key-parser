package com.indeedkeyparser.parse

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CodeExtractorTest {
    @Test
    fun pairs_code_with_label_in_same_container() {
        val root = UiNode(
            null,
            listOf(
                UiNode(null, listOf(UiNode("My Corp"), UiNode("123456"))),
            ),
        )
        assertEquals(Entry("My Corp", "123456"), extractEntry(root))
    }

    @Test
    fun strips_spaces_from_code() {
        val root = UiNode(
            null,
            listOf(
                UiNode(null, listOf(UiNode("Bank"), UiNode("123 456"))),
            ),
        )
        assertEquals(Entry("Bank", "123456"), extractEntry(root))
    }

    @Test
    fun returns_null_when_no_code() {
        val root = UiNode(null, listOf(UiNode(null, listOf(UiNode("Just text")))))
        assertNull(extractEntry(root))
    }
}
