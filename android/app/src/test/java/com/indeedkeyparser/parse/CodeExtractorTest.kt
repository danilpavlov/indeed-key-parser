package com.indeedkeyparser.parse

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CodeExtractorTest {
    // --- Real Indeed Key layout: keyed off elN-* resource ids (see spike note). ---

    /** A single authenticator row as Indeed Key exposes it, chained/nested. */
    private fun row(prefix: String, name: String, account: String, otp: String): UiNode =
        UiNode(name, viewId = "$prefix-tName", children = listOf(
            UiNode(account, viewId = "$prefix-tAccount", children = listOf(
                UiNode(null, viewId = "$prefix-bOpen", children = listOf(
                    UiNode(otp, viewId = "$prefix-tOTP", children = listOf(
                        UiNode(null, viewId = "$prefix-progress", children = listOf(
                            UiNode("Коснитесь для копирования", viewId = "$prefix-tTapHint"),
                        )),
                    )),
                )),
            )),
        ))

    @Test
    fun extracts_revealed_code_by_resource_id() {
        val root = UiNode(null, listOf(row("el0", "test1", "dsds", "851 589")))
        assertEquals(Entry("test1", "851589"), extractEntry(root))
    }

    @Test
    fun returns_null_while_code_is_masked() {
        val root = UiNode(null, listOf(row("el0", "test1", "dsds", "••• •••")))
        assertNull(extractEntry(root))
    }

    @Test
    fun falls_back_to_account_when_name_blank() {
        val root = UiNode(null, listOf(row("el0", "", "dsds", "851 589")))
        assertEquals(Entry("dsds", "851589"), extractEntry(root))
    }

    @Test
    fun picks_the_revealed_row_among_several() {
        val root = UiNode(
            null,
            listOf(
                row("el0", "test1", "dsds", "••• •••"),
                row("el1", "Bank", "acct", "123456"),
            ),
        )
        assertEquals(Entry("Bank", "123456"), extractEntry(root))
    }

    // --- Structural fallback (no resource ids): generic code+label pairing. ---

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
