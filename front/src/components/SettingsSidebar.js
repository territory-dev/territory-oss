import React, { useState } from 'react';
import { Drawer, List, ListItem, ListItemText, Collapse, ListItemIcon } from '@mui/material';
import { ExpandLess, ExpandMore, AccountCircle, Folder, Build, History, Add, Public, Key } from '@mui/icons-material';
import { Link } from 'react-router-dom'

export const SettingsSidebar = ({ repoConfigEnabled, repos, selectedRoute, selectedRepo }) => {
    repos = [...repos]
    repos.sort((a, b) => {
        if (a.public == b.public) {
            if (a.name < b.name) return -1
            else if (a.name > b.name) return 1
            else return 0
        } else {
            if (a.public < b.public) return 1
            else return -1
        }
    })

    const [open, setOpen] = useState({
        repositories: true,
        [selectedRepo]: true,
    });

    const handleClick = (id) => {
        setOpen((prevOpen) => ({ ...prevOpen, [id]: !prevOpen[id] }))
    };

    const renderRepositories = () => {
        return repos.map((repo) => (
            <React.Fragment key={repo.id}>
                <ListItem button onClick={() => handleClick(repo.id)} sx={{ pl: 4 }}>
                    <ListItemIcon>
                        {repo.public
                            ? <Public />
                            : <Folder />}
                    </ListItemIcon>
                    <ListItemText primary={repo.name} />
                    {open[repo.id] ? <ExpandLess /> : <ExpandMore />}
                </ListItem>
                <Collapse in={open[repo.id]} timeout="auto" unmountOnExit>
                    <List component="div" disablePadding>
                        <ListItem
                            className="tRepoConfig"
                            button
                            component={Link}
                            to={`/repos/${repo.id}/config`}
                            sx={{ pl: 8 }}
                            selected={selectedRoute == "buildConfig" && selectedRepo == repo.id}
                        >
                            <ListItemIcon>
                                <Build />
                            </ListItemIcon>
                            <ListItemText primary="Configuration" />
                        </ListItem>
                        <ListItem
                            button
                            component={Link}
                            to={`/repos/${repo.id}/jobs`}
                            sx={{ pl: 8 }}
                            selected={selectedRoute == "jobs" && selectedRepo == repo.id}
                        >
                            <ListItemIcon>
                                <History />
                            </ListItemIcon>
                            <ListItemText primary="Recent builds" />
                        </ListItem>
                    </List>
                </Collapse>
            </React.Fragment>
        ))
    }

    return (
        <Drawer
            className="tSettingsMenu"
            variant="permanent"
            sx={{
                width: 240,
                flexShrink: 0,
                height: '100%',
                '& .MuiDrawer-paper': {
                    width: 240,
                    boxSizing: 'border-box',
                    position: 'static',
                },
            }}

        >
            <List>
                <ListItem
                    button
                    component={Link}
                    to="/account"
                    selected={selectedRoute == "account"}
                >
                    <ListItemIcon>
                        <AccountCircle />
                    </ListItemIcon>
                    <ListItemText primary="My account" />
                </ListItem>
                <ListItem
                    button
                    component={Link}
                    to="/upload-tokens"
                    selected={selectedRoute == "upload-tokens"}
                >
                    <ListItemIcon>
                        <Key />
                    </ListItemIcon>
                    <ListItemText primary="Upload tokens" />
                </ListItem>
                {repoConfigEnabled && <>
                    <ListItem button onClick={() => handleClick('repositories')}>
                        <ListItemIcon>
                            <Folder />
                        </ListItemIcon>
                        <ListItemText primary="Repositories" />
                        {open['repositories'] ? <ExpandLess /> : <ExpandMore />}
                    </ListItem>
                    <Collapse in={open['repositories']} timeout="auto" unmountOnExit>
                        <List component="div" disablePadding>
                            <ListItem
                                button
                                component={Link}
                                to="/repos/new"
                                sx={{ pl: 4 }}
                                selected={selectedRoute == "newBuild"}
                            >
                                <ListItemIcon>
                                    <Add />
                                </ListItemIcon>
                                <ListItemText primary="Add" />
                            </ListItem>
                            {renderRepositories()}
                        </List>
                    </Collapse>
                </>}
            </List>
        </Drawer>
    )
}
