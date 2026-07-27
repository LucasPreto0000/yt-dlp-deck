package com.ytdlpdeck.mobiledownloader

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class DownloadSupportTest {
    @Test
    fun runtimeRejectsASecondConcurrentDownload() {
        DownloadRuntime.begin("job-primary")
        try {
            val error = assertThrows(IllegalStateException::class.java) {
                DownloadRuntime.begin("job-secondary")
            }
            assertEquals("Já existe um download em andamento.", error.message)
        } finally {
            DownloadRuntime.finish("job-primary")
        }
    }

    @Test
    fun runtimeSupportsPauseResumeAndCancel() {
        DownloadRuntime.begin("job-control")
        try {
            assertTrue(DownloadRuntime.pause())
            assertTrue(DownloadRuntime.isPaused())
            assertTrue(DownloadRuntime.resume())
            assertFalse(DownloadRuntime.isPaused())
            assertTrue(DownloadRuntime.cancel())
            assertThrows(DownloadCancelledException::class.java) {
                DownloadRuntime.checkpoint()
            }
        } finally {
            DownloadRuntime.finish("job-control")
        }
    }
}
