package com.yumi.bridge.ui.compose

import android.content.pm.PackageManager
import android.graphics.drawable.Drawable
import android.widget.ImageView
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import com.yumi.bridge.MainActivity
import com.yumi.bridge.ui.theme.YumiTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Filter application list based on search query matching app name or package name.
 */
fun filterAppRules(
    apps: List<MainActivity.AppRuleItem>,
    query: String
): List<MainActivity.AppRuleItem> {
    if (query.isEmpty()) {
        return apps
    }
    return apps.filter {
        it.appName.contains(query, ignoreCase = true) ||
        it.packageName.contains(query, ignoreCase = true)
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppRulesScreen(
    state: HomeUiState,
    onAppModeChanged: (String, String) -> Unit,
    modifier: Modifier = Modifier
) {
    YumiTheme {
        Column(
            modifier = modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            AppRulesSearchCard(
                query = state.appSearchQuery,
                onQueryChange = { state.appSearchQuery = it }
            )

            GlassBackdropWrapper(modifier = Modifier.weight(1f)) {
                val filteredApps = remember(state.installedApps.toList(), state.appSearchQuery) {
                    filterAppRules(state.installedApps, state.appSearchQuery)
                }
                
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(16.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(filteredApps, key = { it.packageName }) { appItem ->
                        AppRuleItemRow(appItem, onAppModeChanged)
                    }
                }
            }
        }
    }
}

@Composable
private fun AppRulesSearchCard(
    query: String,
    onQueryChange: (String) -> Unit
) {
    GlassBackdropWrapper {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "应用规则管理",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = Color(0xFF0F172A),
                modifier = Modifier.padding(bottom = 8.dp)
            )
            OutlinedTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier.fillMaxWidth(),
                placeholder = { Text("搜索应用名或包名") },
                singleLine = true,
                colors = OutlinedTextFieldDefaults.colors(
                    unfocusedContainerColor = Color.Transparent,
                    focusedContainerColor = Color.Transparent,
                    unfocusedBorderColor = Color(0x33000000),
                    focusedBorderColor = Color(0xFF0284C7)
                )
            )
        }
    }
}

@Composable
fun AppRuleItemRow(
    appItem: MainActivity.AppRuleItem,
    onAppModeChanged: (String, String) -> Unit
) {
    var showDialog by remember { mutableStateOf(false) }
    
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(Color(0x0A000000))
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        AppIcon(appItem.packageName, appItem.appName)
        
        Spacer(modifier = Modifier.width(12.dp))
        
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = appItem.appName,
                fontSize = 14.sp,
                fontWeight = FontWeight.Bold,
                color = Color(0xFF0F172A)
            )
            Text(
                text = appItem.packageName,
                fontSize = 11.sp,
                color = Color(0xFF64748B)
            )
        }
        
        ModeButton(
            mode = appItem.currentMode,
            onClick = { showDialog = true }
        )
    }

    if (showDialog) {
        ModeSelectionDialog(
            currentMode = appItem.currentMode,
            onDismiss = { showDialog = false },
            onModeSelected = { newMode ->
                onAppModeChanged(appItem.packageName, newMode)
                showDialog = false
            }
        )
    }
}

@Composable
fun AppIcon(packageName: String, appName: String) {
    val context = LocalContext.current
    var iconDrawable by remember(packageName) { mutableStateOf<Drawable?>(null) }
    
    LaunchedEffect(packageName) {
        iconDrawable = null
        withContext(Dispatchers.IO) {
            try {
                val pm = context.packageManager
                val drawable = pm.getApplicationIcon(packageName)
                iconDrawable = drawable
            } catch (e: PackageManager.NameNotFoundException) {
                // Ignore missing package icon
            }
        }
    }
    
    Box(
        modifier = Modifier
            .size(40.dp)
            .clip(CircleShape)
            .background(Color(0xFFE2E8F0)),
        contentAlignment = Alignment.Center
    ) {
        if (iconDrawable != null) {
            AndroidView(
                factory = { ctx -> 
                    ImageView(ctx).apply { 
                        setImageDrawable(iconDrawable) 
                        scaleType = ImageView.ScaleType.FIT_CENTER
                    } 
                },
                update = { view -> view.setImageDrawable(iconDrawable) },
                modifier = Modifier.fillMaxSize()
            )
        } else {
            val firstLetter = if (appName.isNotEmpty()) appName.substring(0, 1).uppercase() else "A"
            Text(
                text = firstLetter,
                fontSize = 18.sp,
                fontWeight = FontWeight.Bold,
                color = Color(0xFF475569)
            )
        }
    }
}

@Composable
fun ModeButton(mode: String, onClick: () -> Unit) {
    val (text, color) = when (mode.lowercase()) {
        "powersave" -> "省电 (Powersave)" to Color(0xFF16A34A)
        "balance" -> "均衡 (Balance)" to Color(0xFF0284C7)
        "performance" -> "性能 (Performance)" to Color(0xFFEA580C)
        "fast" -> "极速 (Fast)" to Color(0xFFDC2626)
        "fas" -> "FAS 帧感知 (FAS)" to Color(0xFF9333EA)
        else -> "跟随全局 (Default)" to Color(0xFF475569)
    }
    
    Text(
        text = text,
        fontSize = 12.sp,
        fontWeight = FontWeight.Medium,
        color = color,
        modifier = Modifier
            .clip(RoundedCornerShape(8.dp))
            .clickable(onClick = onClick)
            .padding(horizontal = 8.dp, vertical = 4.dp)
    )
}

@Composable
fun ModeSelectionDialog(
    currentMode: String,
    onDismiss: () -> Unit,
    onModeSelected: (String) -> Unit
) {
    val modes = listOf(
        "default" to "跟随全局 (Default)",
        "powersave" to "省电 (Powersave)",
        "balance" to "均衡 (Balance)",
        "performance" to "性能 (Performance)",
        "fast" to "极速 (Fast)",
        "fas" to "FAS 帧感知 (FAS)"
    )
    
    Dialog(onDismissRequest = onDismiss) {
        GlassBackdropWrapper {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(
                    text = "选择调度模式",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
                modes.forEach { (key, label) ->
                    val isSelected = currentMode.equals(key, ignoreCase = true)
                    ModeOptionItem(
                        key = key,
                        label = label,
                        isSelected = isSelected,
                        onModeSelected = onModeSelected
                    )
                }
            }
        }
    }
}

@Composable
private fun ModeOptionItem(
    key: String,
    label: String,
    isSelected: Boolean,
    onModeSelected: (String) -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .clickable { onModeSelected(key) }
            .background(if (isSelected) Color(0x200284C7) else Color.Transparent)
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(
            text = label,
            fontSize = 14.sp,
            fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
            color = if (isSelected) Color(0xFF0284C7) else Color(0xFF0F172A)
        )
    }
}

