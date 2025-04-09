import moment from 'moment'

import { FontAwesomeIcon } from '@fortawesome/react-fontawesome'
import { faGlobe } from '@fortawesome/free-solid-svg-icons'

import React, { useState } from 'react'
import { Link } from 'react-router-dom'
import Box from '@mui/material/Box'
import List from '@mui/material/List'
import ListItem from '@mui/material/ListItem'
import ListItemButton from '@mui/material/ListItemButton'
import IconButton from '@mui/material/IconButton'
import DeleteIcon from '@mui/icons-material/Delete'

import styles from './MapsList.module.css'

export const MapsList = ({ maps, setRemoveMap }) => {
    const MAX_VISIBLE = 6
    const [isExpanded, setExpanded] = useState(false)
    const visibleMaps = isExpanded ? maps : maps.slice(0, MAX_VISIBLE)

    return <Box sx={{ width: '100%', maxWidth: 720, marginBottom: '40px', bgcolor: 'background.paper' }}>
        <List className="tMapList">
            {visibleMaps.map(({ id, display_name, last_changed, repo_name, public: is_public }) => (
                    <ListItem
                        secondaryAction={
                            <IconButton
                                edge="end"
                                aria-label="delete"
                                onClick={() => setRemoveMap({ id, display_name })}
                            >
                                <DeleteIcon />
                            </IconButton>
                        }
                        key={id}
                    >
                        <ListItemButton
                            className={styles.mapItem}
                            component={Link}
                            to={`/maps/${id}`}
                        >
                            <div className={styles.mapName}>
                                {display_name}
                            </div>
                            <div className={styles.mapDetails}>
                                <div className={styles.mapLastChanged}>{moment(last_changed).fromNow()}</div>
                                <div className={styles.mapRepoName}>{repo_name}</div>
                                <div className={styles.mapSharingState}>{is_public && (
                                    <>
                                        Shared
                                        <FontAwesomeIcon className={styles.shareIcon} size="xs" icon={faGlobe}/>
                                    </>
                                )}</div>
                            </div>
                        </ListItemButton>
                    </ListItem>
                ))
            }

            {!isExpanded && (maps.length > MAX_VISIBLE) &&
                <ListItem>
                    <ListItemButton onClick={() => setExpanded(true)}
                        sx={{
                            justifyContent: 'center',
                            textAlign: 'center',
                            color: 'rgb(105, 105, 105)',
                        }}
                    >
                        See earlier
                    </ListItemButton>
                </ListItem>}
        </List>
    </Box>
}
